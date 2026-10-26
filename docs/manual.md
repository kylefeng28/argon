ARGON(1)                        User Manual                        ARGON(1)

NAME
    argon - a fantasy console

SYNOPSIS
    argon [--disk PATH] [--portable]
    argon run PROGRAM [--headless --frames N --screenshot FILE --keys SCRIPT --text]
    argon shell --headless [--frames N --screenshot FILE --keys SCRIPT --text]
    argon check PROGRAM
    argon docs [--compact]

DESCRIPTION
    Argon boots into its own small computer: a shell, an editor, a scaled
    640x400 display with an 80x25 text layer, a sandboxed disk and a Luau
    runtime. Programs are single .lua files or cart folders. Nothing a program
    does can reach outside the virtual disk.

    Data lives in ~/.argon/ (disk/, export/, config.toml, history.txt).
    --disk PATH uses another folder as the disk; --portable keeps everything
    in argon-data/ next to the executable.

KEYS (anywhere)
    F11, Alt+Enter   Toggle fullscreen
    Esc              Stop the running program and return to the shell (or to
                     the editor if the program was started with F5)

THE SHELL
    The prompt is ARGON> (ARGON:FOLDER> inside a folder). Commands are
    case-insensitive. Anything that is not a command is evaluated as Lua:
    expressions print their value, statements run, drawing functions draw on
    the shell screen. Variables persist between lines until VARCLEAR.

    Tab completes commands, program names and function names. Up/Down walk
    the history. Ctrl+U clears the line.

    DIR [pattern]      List programs and files. <CART> marks a program folder.
                       Wildcards: DIR *.lua. Alias: LS.
    RUN [name]         Run a program; no name re-runs the last one. Esc stops.
    EDIT [name]        Open a program in the editor (creates it if missing).
    NEW name           Create a program from a template and open it.
    TYPE name          Print a file. Alias: CAT.
    COPY from to       Copy a file or folder.
    REN from to        Rename a file or folder.
    DEL name           Delete a file or folder after Y/N confirmation.
    CD folder          Change folder. CD .. goes up, CD alone goes to the root.
    MKDIR name         Create a folder.
    CLS                Clear the screen. Alias: CLEAR.
    VARCLEAR           Forget all variables and functions defined at the prompt.
    HELP [topic]       Help on a command, a function (HELP fill_rect), or a
                       category (HELP draw, HELP sound, ...).
    THEME [name]       Colors: grid (default), phosphor, amber, blue, lcd.
    FILTER [name]      Display effect: none, glow, glass, scanlines,
                       neon (default: glow + glass + vignette), crt.
    FONT [name]        Console font: tech (Share Tech Mono, default), proggy
                       (ProggyClean), pixel (Unscii bitmap). All three
                       settings are saved in config.toml.
    VER                Show the version.
    EXPORT name        Copy a program to ~/.argon/export/ for sharing.
    IMPORT path        Copy a .lua file or program folder from the host disk
                       into the current folder.
    EXIT               Quit Argon.

    Program names resolve as name, name.lua, or name/main.lua, case-
    insensitively. DEMOS/ is read-only; COPY a demo to edit it.

THE EDITOR
    EDIT or NEW opens a full-screen editor with Lua syntax highlighting, line
    numbers and a status line.

    F1               Catalog: every function with its signature and a one-line
                     description. Type to filter, Enter inserts a template.
    F2               Control menu: if, for, while, function, main loop, ...
    F4               Draw and Image functions
    F6               Math and System functions
    F7               Text and Input functions
    F8               File functions
    F9               Sound functions
    F5, Ctrl+R       Save and run. An error returns to the editor with the
                     cursor on the offending line and the message in the status
                     line.
    F10              Save and return to the shell.
    Esc              Return to the shell (asks to save if modified). Also clears
                     a selection or pending template placeholders.
    Ctrl+S           Save.
    Ctrl+Z, Ctrl+Y   Undo, redo.
    Ctrl+F, F3       Find, find next.
    Ctrl+H           Replace all.
    Ctrl+G           Go to line.
    Ctrl+A           Select all.   Shift+arrows select.
    Ctrl+C/X/V       Copy, cut, paste (whole line when nothing is selected).
    Ctrl+D           Duplicate line.
    Ctrl+/           Toggle comment.
    Alt+Up/Down      Move line.
    Tab              Indent, or jump to the next template placeholder.
    Shift+Tab        Unindent.

    Inserted templates look like fill_rect(x, y, w, h) with each placeholder
    selected in turn; type over it and press Tab for the next one.

PROGRAMS
    A program runs from top to bottom. Draw, then call flip() to show the
    frame and read input; loop as long as you like. Text output goes to the
    text layer (print, locate, put_char), pixels to the framebuffer
    (fill_rect, pixel, draw_sprite, ...). The framebuffer persists until cls().

        alt_screen()                     -- optional: keep the shell clean
        local x = 0
        while true do
          cls(rgb(10, 10, 30))
          fill_circle(x, 200, 20, rgb(255, 200, 0))
          x = (x + 2) % 640
          if button("a") then beep(880, 0.05) end
          flip()
        end

    A program may switch resolution at any time: set_resolution("retro")
    (320x240), "tiny", "classic", "hd" or "WxH". Combined with alt_screen()
    the shell comes back untouched at 640x400 when the program ends.

    Settings can be given in a comment header of a single file:

        -- argon: resolution = "retro"      (tiny 128x128, retro 320x240,
        -- argon: fps = 30                   classic 640x400, hd 1280x720, WxH)

    or in cart.toml next to main.lua:

        title = "Shooter"
        author = "you"
        resolution = "classic"
        fps = 60
        filter = "crt"          # optional display effect while running

    Pad buttons (button("a") etc.) map to the keyboard: arrows, Z=a, X=b,
    A=x, S=y, Q=l, W=r, Enter=start, Shift=select. Change them under
    [keymap.pad] in ~/.argon/config.toml, e.g. a = ["z", "k"].

    Files: read_file/write_file see only the program's own folder; save/load
    keep small values in SAVE/ between runs. require("name") loads name.lua
    from the program folder.

    Colors are rgb(r, g, b [, a]) or hsv(h, s, v) values; every drawing
    function takes an optional color, defaulting to color().

    Errors look like

        ERROR in pong.lua:12: attempt to call a nil value
          Did you mean fill_rect?
            12 | fil_rect(10, 10, 50, 50)

    Lua is Luau: standard string, table, math, utf8 and coroutine libraries;
    no io, os, debug or loadstring. pcall cannot catch Esc.

FUNCTIONS
    Full reference: argon docs (Markdown) or argon docs --compact. In the
    console: HELP <name>, or F1 in the editor.

    System   flip cls wait time delta_time frame fps screen_size
             set_resolution alt_screen main_screen exit rgb hsv color
    Text     print write locate text_color put_char get_char text_size
             cursor clear_text
    Input    input wait_key read_key button button_pressed button_released
             key key_pressed key_released
    Draw     pixel line line_width rect fill_rect circle fill_circle ellipse
             fill_ellipse triangle fill_triangle polygon fill_polygon
             draw_text clip camera blend
    Image    load_image new_image image_size draw_image draw_sprite
             draw_image_quad set_target
    Sound    beep tone note_freq load_sound play_sound set_voice stop_sound
             stop_all_sounds set_volume
    File     read_file write_file append_file file_exists delete_file
             list_files save load require
    Math     random seed clamp lerp round sign dist angle
             (plus the standard math library)

FILES
    ~/.argon/disk/            The virtual disk (what DIR shows)
    ~/.argon/disk/DEMOS/      Demo programs, read-only, reinstalled at start
    ~/.argon/disk/SAVE/       save()/load() data, one JSON file per program
    ~/.argon/export/          EXPORT output
    ~/.argon/config.toml      theme, filter, font, window_scale, fullscreen,
                              keymap, editor
    ~/.argon/history.txt      shell history

HEADLESS
    argon shell --headless --keys "help{enter}" --text --screenshot out.png
    argon run DEMOS/pong --headless --frames 120 --screenshot pong.png

    Key scripts type plain characters; braces hold special keys and
    directives: {enter} {esc} {tab} {f5} {up} {ctrl+s} {shift+right}
    {wait:30} (frames) {hold:z} {release:z}.

SEE ALSO
    docs/agent-guide.md     a compact description for code-generating models
    docs/api-sketch.md      the API design, including what is not built yet
    docs/requirements.md    product requirements and decisions
