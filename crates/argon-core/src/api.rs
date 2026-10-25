//! The API table: the single source of truth for documentation, editor menus,
//! the Catalog, `HELP`, completion and "did you mean" hints (NFR-9).

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Category {
    System,
    Control,
    Text,
    Input,
    Draw,
    Image,
    Gpu,
    Sound,
    File,
    Math,
}

impl Category {
    pub fn name(self) -> &'static str {
        match self {
            Category::System => "System",
            Category::Control => "Control",
            Category::Text => "Text",
            Category::Input => "Input",
            Category::Draw => "Draw",
            Category::Image => "Image",
            Category::Gpu => "GPU",
            Category::Sound => "Sound",
            Category::File => "File",
            Category::Math => "Math",
        }
    }

    pub const ALL: [Category; 10] = [
        Category::System,
        Category::Control,
        Category::Text,
        Category::Input,
        Category::Draw,
        Category::Image,
        Category::Gpu,
        Category::Sound,
        Category::File,
        Category::Math,
    ];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiEntry {
    pub name: &'static str,
    pub category: Category,
    /// e.g. "fill_circle(x, y, r [, color])"
    pub signature: &'static str,
    pub doc: &'static str,
    pub example: &'static str,
    /// Snippet inserted by the editor; `${name}` marks placeholders.
    pub snippet: &'static str,
}

#[derive(Default)]
pub struct ApiTable {
    entries: Vec<ApiEntry>,
}

impl ApiTable {
    pub fn new() -> ApiTable {
        ApiTable::default()
    }

    pub fn add(&mut self, e: ApiEntry) {
        debug_assert!(!self.entries.iter().any(|x| x.name == e.name), "duplicate api entry {}", e.name);
        self.entries.push(e);
    }

    pub fn entries(&self) -> &[ApiEntry] {
        &self.entries
    }

    pub fn get(&self, name: &str) -> Option<&ApiEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    pub fn in_category(&self, c: Category) -> impl Iterator<Item = &ApiEntry> {
        self.entries.iter().filter(move |e| e.category == c)
    }

    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.entries.iter().map(|e| e.name)
    }

    /// Closest name by edit distance (<= 2), for "did you mean".
    pub fn suggest(&self, name: &str) -> Option<&'static str> {
        let mut best: Option<(usize, &'static str)> = None;
        for e in &self.entries {
            let d = levenshtein(name, e.name);
            if d <= 2 && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, e.name));
            }
        }
        best.map(|(_, n)| n)
    }

    pub fn sorted(&self) -> Vec<&ApiEntry> {
        let mut v: Vec<&ApiEntry> = self.entries.iter().collect();
        v.sort_by_key(|e| e.name);
        v
    }
}

pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Snippet placeholder parsing: returns the literal text and the byte ranges
/// (in the output) of each `${...}` placeholder.
pub fn expand_snippet(snippet: &str) -> (String, Vec<(usize, usize)>) {
    let mut out = String::new();
    let mut ranges = Vec::new();
    let mut rest = snippet;
    while let Some(i) = rest.find("${") {
        out.push_str(&rest[..i]);
        let after = &rest[i + 2..];
        if let Some(j) = after.find('}') {
            let start = out.len();
            out.push_str(&after[..j]);
            ranges.push((start, out.len()));
            rest = &after[j + 1..];
        } else {
            out.push_str(&rest[i..]);
            rest = "";
        }
    }
    out.push_str(rest);
    (out, ranges)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_placeholders() {
        let (s, r) = expand_snippet("fill_rect(${x}, ${y}, ${w}, ${h})");
        assert_eq!(s, "fill_rect(x, y, w, h)");
        assert_eq!(r, vec![(10, 11), (13, 14), (16, 17), (19, 20)]);
    }
}
