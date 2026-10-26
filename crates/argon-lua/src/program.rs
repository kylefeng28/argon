//! The program thread and its handle (technical design section 2.1).

use crate::machine::{Flip, FlipError, FrameAck, FrameOut, MachineOptions};
use crate::runtime::{Outcome, Runtime};
use crate::ArgonError;
use argon_core::vfs::Jail;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TryRecvError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Everything needed to start a program.
#[derive(Clone, Debug)]
pub struct ProgramSpec {
    pub main_name: String,
    pub source: String,
    pub jail: Jail,
    pub save_jail: Option<Jail>,
    pub options: MachineOptions,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProgramExit {
    Finished,
    Break,
    Error(ArgonError),
}

/// Program-side end of the frame protocol.
struct ChannelFlip {
    frames: SyncSender<FrameOut>,
    acks: Receiver<FrameAck>,
    break_flag: Arc<AtomicBool>,
}

impl Flip for ChannelFlip {
    fn flip(&mut self, out: FrameOut) -> Result<FrameAck, FlipError> {
        if self.frames.send(out).is_err() {
            return Err(FlipError::Break);
        }
        loop {
            if self.break_flag.load(Ordering::Relaxed) {
                return Err(FlipError::Break);
            }
            match self.acks.recv_timeout(Duration::from_millis(20)) {
                Ok(ack) => return Ok(ack),
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => return Err(FlipError::Break),
            }
        }
    }
}

/// Main-thread handle to a running program.
pub struct ProgramHandle {
    frames: Receiver<FrameOut>,
    acks: Sender<FrameAck>,
    exit: Receiver<(ProgramExit, Option<FrameOut>)>,
    break_flag: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    /// A frame received but not yet acknowledged.
    pending: Option<FrameOut>,
    frame_no: u64,
    last_ack: Instant,
    exited: Option<ProgramExit>,
    /// Unflipped drawing and text left over when the program ended.
    final_frame: Option<FrameOut>,
}

pub fn spawn_program(spec: ProgramSpec) -> ProgramHandle {
    let (frame_tx, frame_rx) = mpsc::sync_channel::<FrameOut>(1);
    let (ack_tx, ack_rx) = mpsc::channel::<FrameAck>();
    let (exit_tx, exit_rx) = mpsc::channel::<(ProgramExit, Option<FrameOut>)>();
    let break_flag = Arc::new(AtomicBool::new(false));
    let flag2 = break_flag.clone();

    let join = std::thread::Builder::new()
        .name("argon-program".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let flip = ChannelFlip { frames: frame_tx, acks: ack_rx, break_flag: flag2.clone() };
            let result = Runtime::new(spec.options, spec.jail, spec.save_jail, flag2, Box::new(flip));
            let (exit, final_frame) = match result {
                Err(e) => (ProgramExit::Error(ArgonError::simple(format!("failed to start Lua: {e}"))), None),
                Ok(mut rt) => {
                    let exit = match rt.run_main(&spec.main_name, &spec.source) {
                        Outcome::Finished => ProgramExit::Finished,
                        Outcome::Break => ProgramExit::Break,
                        Outcome::Error(e) => ProgramExit::Error(e),
                    };
                    let frame = {
                        let mut m = rt.machine.borrow_mut();
                        FrameOut { cmds: std::mem::take(&mut m.cmds), grid: m.grid.clone(), resolution: m.pending_resolution.take(), fps: m.fps, alt_screen: m.alt_screen, audio: std::mem::take(&mut m.audio) }
                    };
                    (exit, Some(frame))
                }
            };
            let _ = exit_tx.send((exit, final_frame));
        })
        .expect("spawn program thread");

    ProgramHandle {
        frames: frame_rx,
        acks: ack_tx,
        exit: exit_rx,
        break_flag,
        join: Some(join),
        pending: None,
        frame_no: 0,
        last_ack: Instant::now(),
        exited: None,
        final_frame: None,
    }
}

impl ProgramHandle {
    /// Non-blocking: fetch a newly flipped frame if the program produced one.
    pub fn poll_frame(&mut self) -> Option<&FrameOut> {
        if self.pending.is_none() {
            match self.frames.try_recv() {
                Ok(f) => self.pending = Some(f),
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => {}
            }
        }
        self.pending.as_ref()
    }

    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Target fps of the pending frame, if any.
    pub fn pending_fps(&self) -> Option<u32> {
        self.pending.as_ref().map(|f| f.fps)
    }

    /// Acknowledge the pending frame, letting the program continue. `dt`
    /// overrides the measured frame time (headless runs use a fixed step).
    pub fn ack(&mut self, input: argon_core::InputSnapshot, fixed_dt: Option<f64>) {
        if self.pending.take().is_some() {
            let now = Instant::now();
            let dt = fixed_dt.unwrap_or_else(|| now.duration_since(self.last_ack).as_secs_f64().min(0.25));
            self.last_ack = now;
            self.frame_no += 1;
            let _ = self.acks.send(FrameAck { input, dt: dt.min(0.25), frame: self.frame_no });
        }
    }

    /// Time since the last ack, for frame pacing.
    pub fn since_last_ack(&self) -> Duration {
        self.last_ack.elapsed()
    }

    /// Ask the program to stop (Esc).
    pub fn request_break(&self) {
        self.break_flag.store(true, Ordering::Relaxed);
    }

    /// Non-blocking check whether the program has ended.
    pub fn poll_exit(&mut self) -> Option<&ProgramExit> {
        if self.exited.is_none() {
            if let Ok((e, frame)) = self.exit.try_recv() {
                self.exited = Some(e);
                self.final_frame = frame;
            }
        }
        self.exited.as_ref()
    }

    /// The program's leftover frame (drawing done after the last `flip()`,
    /// plus its text layer). Available once `poll_exit` returned.
    pub fn take_final_frame(&mut self) -> Option<FrameOut> {
        self.final_frame.take()
    }

    /// Block (bounded) until the program thread has ended.
    pub fn finish(mut self, timeout: Duration) -> (ProgramExit, Option<FrameOut>) {
        self.request_break();
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(e) = self.poll_exit() {
                let e = e.clone();
                if let Some(j) = self.join.take() {
                    let _ = j.join();
                }
                return (e, self.final_frame.take());
            }
            if Instant::now() > deadline {
                // Leave the thread detached rather than hang the UI.
                log::warn!("program thread did not stop in time; detaching");
                return (ProgramExit::Break, None);
            }
            // Let any pending frame through so a program blocked in flip() wakes.
            self.pending = None;
            while self.frames.try_recv().is_ok() {}
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for ProgramHandle {
    fn drop(&mut self) {
        self.request_break();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon_core::InputSnapshot;

    fn spec(src: &str) -> (ProgramSpec, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let jail = Jail::new(dir.path().to_path_buf());
        (
            ProgramSpec {
                main_name: "main.lua".into(),
                source: src.into(),
                jail,
                save_jail: None,
                options: MachineOptions::default(),
            },
            dir,
        )
    }

    #[test]
    fn frames_flow_and_program_finishes() {
        let (s, _d) = spec("for i = 1, 3 do fill_rect(i, 0, 1, 1) flip() end");
        let mut h = spawn_program(s);
        let mut frames = 0;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if h.poll_frame().is_some() {
                frames += 1;
                h.ack(InputSnapshot::default(), None);
            }
            if let Some(e) = h.poll_exit() {
                assert_eq!(*e, ProgramExit::Finished);
                break;
            }
            assert!(Instant::now() < deadline, "timed out after {frames} frames");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(frames, 3);
    }

    #[test]
    fn break_stops_tight_loop_quickly() {
        let (s, _d) = spec("while true do end");
        let h = spawn_program(s);
        std::thread::sleep(Duration::from_millis(50));
        let t = Instant::now();
        let (e, _) = h.finish(Duration::from_secs(2));
        assert_eq!(e, ProgramExit::Break);
        assert!(t.elapsed() < Duration::from_millis(500), "{:?}", t.elapsed());
    }

    #[test]
    fn break_stops_program_blocked_in_flip() {
        let (s, _d) = spec("while true do flip() end");
        let mut h = spawn_program(s);
        let deadline = Instant::now() + Duration::from_secs(2);
        while h.poll_frame().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(h.has_pending());
        let t = Instant::now();
        assert_eq!(h.finish(Duration::from_secs(2)).0, ProgramExit::Break);
        assert!(t.elapsed() < Duration::from_millis(500));
    }
}
