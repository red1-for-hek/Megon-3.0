//! Windowed file editor — behavioral parity Rust port of the TypeScript
//! `WindowedFile` and `str_replace_editor` from SWEagent.
//!
//! Key design decisions derived from reading `src/tools/windowed-file.ts`:
//!
//! * `_firstLine` (TS) == `window_start` here — always **0-indexed**.
//! * `_window` (TS) == `window_size` — number of lines visible at once.
//! * `lineRange` (TS) computes `end = min(start + window - 1, nLines - 1)`.
//! * `goto(n, 'center')` (TS): `firstLine = max(0, n - floor(window/2))` —
//!   note `n` is 0-indexed in TS internal representation.
//! * `scroll(lines)` (TS) just adds to `firstLine`; the `firstLine` setter
//!   clamps to `[0, nLines-1]`.
//! * `getWindowText` with `lineNumbers=true` uses `(i+1):{line}` format
//!   (1-indexed display, no padding).
//! * `insert(text, lineNumber)` inserts **after** the given 0-indexed position
//!   (i.e. `splice(index, 0, ...)` where `index = lineNumber`).
//! * `replace` replaces every line that *contains* `oldText`, not whole-file
//!   substring replace.

use forge_types::ForgeError;

// ──────────────────────────────────────────────────────────────────────────────
// WindowedFile
// ──────────────────────────────────────────────────────────────────────────────

/// In-memory windowed view of a text file.
///
/// All public line-number arguments/returns use **1-indexed** numbering
/// (matching the TS format output) unless explicitly documented otherwise.
/// Internally `window_start` is always 0-indexed.
pub struct WindowedFile {
    lines: Vec<String>,
    /// 0-indexed index of the first visible line.
    window_start: usize,
    /// Number of lines in the visible window.
    window_size: usize,
    /// Multiplier used by `goto(n, top)` — matches TS `offsetMultiplier`.
    pub offset_multiplier: f64,
}

impl WindowedFile {
    /// Create a new `WindowedFile` with a custom window size.
    pub fn new(window_size: usize) -> Self {
        Self {
            lines: Vec::new(),
            window_start: 0,
            window_size: window_size.max(1),
            offset_multiplier: 0.25,
        }
    }

    /// Create with the default window size of 100 lines.
    pub fn with_default_window() -> Self {
        Self::new(100)
    }

    // ── content ──────────────────────────────────────────────────────────────

    /// Load content from a string (splits on `'\n'`).
    ///
    /// Mirrors the TS `loadFile` logic: an empty or whitespace-only file
    /// becomes a single empty line.
    pub fn set_content(&mut self, content: &str) {
        let mut lines: Vec<String> = content.split('\n').map(str::to_owned).collect();
        // split('\n') always produces at least one element for non-empty content
        if content.trim().is_empty() {
            lines = vec![String::new()];
        }
        self.lines = lines;
        // Re-clamp window_start after new content
        self.clamp_window_start();
    }

    /// Reconstruct the full file content joined by newlines.
    pub fn get_content(&self) -> String {
        self.lines.join("\n")
    }

    /// Total number of lines in the file.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    // ── window bounds ─────────────────────────────────────────────────────────

    /// `[start, end]` inclusive, 0-indexed.  Mirrors TS `lineRange`.
    fn line_range(&self) -> (usize, usize) {
        let start = self.window_start;
        let end = (self.window_start + self.window_size).saturating_sub(1).min(self.line_count().saturating_sub(1));
        (start, end)
    }

    /// Clamp `window_start` so the window always stays within file bounds.
    /// Mirrors the TS `firstLine` setter: `max(0, min(value, nLines - 1))`.
    fn clamp_window_start(&mut self) {
        let n = self.line_count();
        if n == 0 {
            self.window_start = 0;
        } else {
            self.window_start = self.window_start.min(n - 1);
        }
    }

    // ── navigation ────────────────────────────────────────────────────────────

    /// Jump to a **1-indexed** line number, centering the window on it.
    ///
    /// TS `goto(lineNumber, 'center')`:
    /// ```text
    /// firstLine = max(0, lineNumber - floor(window / 2))
    /// ```
    /// `lineNumber` in TS is **0-indexed** internally, so for a 1-indexed
    /// public API we convert: `internal = line - 1`.
    pub fn goto(&mut self, line: usize) {
        // Convert 1-indexed → 0-indexed
        let line0 = line.saturating_sub(1);
        let half = self.window_size / 2;
        self.window_start = line0.saturating_sub(half);
        self.clamp_window_start();
    }

    /// Jump using 'top' mode (offset from top), **1-indexed** line number.
    ///
    /// TS `goto(lineNumber, 'top')`:
    /// ```text
    /// firstLine = max(0, lineNumber - floor(window * offsetMultiplier))
    /// ```
    pub fn goto_top(&mut self, line: usize) {
        let line0 = line.saturating_sub(1);
        let offset = (self.window_size as f64 * self.offset_multiplier).floor() as usize;
        self.window_start = line0.saturating_sub(offset);
        self.clamp_window_start();
    }

    /// Scroll by `delta` lines (positive = down, negative = up).
    /// Mirrors TS `scroll(lines)` which sets `firstLine = _firstLine + lines`.
    pub fn scroll(&mut self, delta: i64) {
        let new_start = (self.window_start as i64 + delta).max(0) as usize;
        self.window_start = new_start;
        self.clamp_window_start();
    }

    /// Scroll up by `window_size / 2` lines.
    pub fn scroll_up(&mut self) {
        let amount = (self.window_size / 2) as i64;
        self.scroll(-amount);
    }

    /// Scroll down by `window_size / 2` lines.
    pub fn scroll_down(&mut self) {
        let amount = (self.window_size / 2) as i64;
        self.scroll(amount);
    }

    // ── current position accessors ────────────────────────────────────────────

    /// The 1-indexed line number of the first visible line.
    pub fn current_line(&self) -> usize {
        self.window_start + 1
    }

    /// The 1-indexed line number of the last visible line (clamped to file length).
    pub fn end_line(&self) -> usize {
        let (_, end) = self.line_range();
        end + 1
    }

    /// Raw 0-indexed window start (for tests that mirror TS `firstLine`).
    pub fn first_line(&self) -> usize {
        self.window_start
    }

    /// Set window start directly (0-indexed), clamping to bounds.
    /// Mirrors the TS `firstLine` setter.
    pub fn set_first_line(&mut self, value: usize) {
        self.window_start = value;
        self.clamp_window_start();
    }

    /// Window size accessor.
    pub fn window(&self) -> usize {
        self.window_size
    }

    // ── formatted output ──────────────────────────────────────────────────────

    /// Produce the window as a formatted string.
    ///
    /// Equivalent to TS `getWindowText(header, lineNumbers, footer)`.
    ///
    /// With `header=true` prepends `[File: <path> (<n> lines total)]` and
    /// `(<start> more lines above)`.  With `footer=true` appends
    /// `(<remaining> more lines below)`.  With `line_numbers=true` prefixes
    /// each line with `{1-indexed line num}:{content}`.
    pub fn get_window_text(
        &self,
        header: Option<&str>,
        line_numbers: bool,
        footer: bool,
    ) -> String {
        let (start, end) = self.line_range();
        let mut parts: Vec<String> = Vec::new();

        if let Some(path) = header {
            parts.push(format!("[File: {} ({} lines total)]", path, self.line_count()));
            if start > 0 {
                parts.push(format!("({} more lines above)", start));
            }
        }

        for i in start..=end {
            if line_numbers {
                parts.push(format!("{}:{}", i + 1, self.lines[i]));
            } else {
                parts.push(self.lines[i].clone());
            }
        }

        if footer {
            let remaining = self.line_count().saturating_sub(end + 1);
            if remaining > 0 {
                parts.push(format!("({} more lines below)", remaining));
            }
        }

        parts.join("\n")
    }

    /// Produce a tab-separated line-numbered window (`{line_num}\t{content}`).
    /// Used by the agent tool output format.
    pub fn format_window(&self) -> String {
        let (start, end) = self.line_range();
        let mut parts: Vec<String> = Vec::new();
        for i in start..=end {
            parts.push(format!("{}\t{}", i + 1, self.lines[i]));
        }
        parts.join("\n")
    }

    // ── text search ───────────────────────────────────────────────────────────

    /// Find all lines containing `pattern` (case-insensitive substring match).
    /// Returns `(1-indexed line number, line content)` pairs.
    pub fn search(&self, pattern: &str) -> Vec<(usize, &str)> {
        let lower = pattern.to_lowercase();
        self.lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.to_lowercase().contains(&lower))
            .map(|(i, line)| (i + 1, line.as_str()))
            .collect()
    }

    /// Find all 1-indexed line numbers where lines *contain* `search_text`.
    /// Mirrors TS `findAllOccurrences(searchText, zeroBased=false)`.
    pub fn find_all_occurrences(&self, search_text: &str, zero_based: bool) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.contains(search_text))
            .map(|(i, _)| if zero_based { i } else { i + 1 })
            .collect()
    }

    // ── text editing ──────────────────────────────────────────────────────────

    /// Replace text within the current window (first occurrence only).
    ///
    /// Mirrors TS `replaceInWindow(oldText, newText)`.  After replacement,
    /// adjusts `window_start` backwards by `floor(window * offsetMultiplier) + 1`.
    pub fn replace_in_window(&mut self, old_text: &str, new_text: &str) -> Result<(), ForgeError> {
        let (start, end) = self.line_range();
        let window_content = self.lines[start..=end].join("\n");

        if !window_content.contains(old_text) {
            return Err(ForgeError::Environment(format!(
                "Text not found in current window: {}",
                old_text
            )));
        }

        let new_window_content = window_content.replacen(old_text, new_text, 1);
        let new_lines: Vec<String> = new_window_content.split('\n').map(str::to_owned).collect();

        let mut content = self.lines[..start].to_vec();
        content.extend(new_lines);
        content.extend_from_slice(&self.lines[end + 1..]);
        self.lines = content;

        // Adjust window backwards
        let offset = (self.window_size as f64 * self.offset_multiplier).floor() as usize + 1;
        self.window_start = self.window_start.saturating_sub(offset);
        self.clamp_window_start();

        Ok(())
    }

    /// Replace every occurrence of `old_text` in lines that contain it.
    ///
    /// Mirrors TS `replace(oldText, newText)` which does per-line regex replace.
    /// Returns `(n_replacements, first_replaced_line_1indexed)`.
    pub fn replace(
        &mut self,
        old_text: &str,
        new_text: &str,
    ) -> (usize, Option<usize>) {
        let mut n_replacements = 0usize;
        let mut first_replaced_line: Option<usize> = None;

        for (i, line) in self.lines.iter_mut().enumerate() {
            if line.contains(old_text) {
                *line = line.replace(old_text, new_text);
                n_replacements += 1;
                if first_replaced_line.is_none() {
                    first_replaced_line = Some(i + 1);
                }
            }
        }

        (n_replacements, first_replaced_line)
    }

    /// Insert `text` after the given 0-indexed position.
    ///
    /// Mirrors TS `insert(text, lineNumber)` which calls
    /// `splice(lineNumber, 0, ...linesToInsert)` — i.e. inserts *at* index
    /// `lineNumber`, pushing everything after it down.
    pub fn insert_at(&mut self, index: usize, text: &str) -> usize {
        let to_insert: Vec<String> = text.split('\n').map(str::to_owned).collect();
        let n_added = to_insert.len();
        let insert_pos = index.min(self.lines.len());
        self.lines.splice(insert_pos..insert_pos, to_insert);
        n_added
    }

    /// Take a snapshot of the current line content for undo purposes.
    pub(crate) fn snapshot(&self) -> Vec<String> {
        self.lines.clone()
    }

    /// Restore lines from a snapshot. Window is reclamped.
    pub(crate) fn restore(&mut self, snapshot: Vec<String>) {
        self.lines = snapshot;
        self.clamp_window_start();
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// StrReplaceEditor
// ──────────────────────────────────────────────────────────────────────────────

/// Simple in-memory editor wrapping `WindowedFile`.
///
/// Provides `str_replace` (unique-occurrence-enforced) and `insert` semantics
/// matching the TS `str_replace_editor` tool.
pub struct StrReplaceEditor {
    windowed_file: WindowedFile,
    edit_history: Vec<Vec<String>>,
}

impl StrReplaceEditor {
    pub fn new(window_size: usize) -> Self {
        Self {
            windowed_file: WindowedFile::new(window_size),
            edit_history: Vec::new(),
        }
    }

    pub fn set_content(&mut self, content: &str) {
        self.windowed_file.set_content(content);
    }

    /// Reconstruct the full content from the current lines.
    pub fn get_content(&self) -> String {
        self.windowed_file.get_content()
    }

    /// Replace the **first and only** occurrence of `old_str` with `new_str`.
    ///
    /// Returns an error if `old_str` is not found or appears more than once.
    pub fn str_replace(&mut self, old_str: &str, new_str: &str) -> Result<(), ForgeError> {
        let content = self.windowed_file.get_content();
        let count = content.matches(old_str).count();
        if count == 0 {
            return Err(ForgeError::Environment(
                "str_replace: old_str not found in file".into(),
            ));
        }
        if count > 1 {
            return Err(ForgeError::Environment(format!(
                "str_replace: old_str appears {} times (must be unique)",
                count
            )));
        }
        let replacement_offset = content
            .find(old_str)
            .ok_or_else(|| ForgeError::Environment("str_replace: internal error".into()))?;
        let new_content = format!(
            "{}{}{}",
            &content[..replacement_offset],
            new_str,
            &content[replacement_offset + old_str.len()..],
        );
        // Push snapshot AFTER computing new content, BEFORE mutating
        self.edit_history.push(self.windowed_file.snapshot());
        self.set_content(&new_content);
        // Move window to start of replacement
        let first_replaced_line = content[..replacement_offset].matches('\n').count() + 1;
        self.windowed_file.goto(first_replaced_line);
        Ok(())
    }

    /// Insert `content` after the given **1-indexed** line number.
    ///
    /// `after_line = 0` inserts at the very beginning.
    pub fn insert(&mut self, after_line: usize, content: &str) -> Result<(), ForgeError> {
        let n = self.windowed_file.line_count();
        if after_line > n {
            return Err(ForgeError::Environment(format!(
                "insert: line {} out of range (file has {} lines)",
                after_line,
                n
            )));
        }

        // Save history before mutating
        self.edit_history.push(self.windowed_file.snapshot());

        // `after_line` is 1-indexed; insert position (0-indexed) = after_line
        // (i.e. right after line `after_line`).
        self.windowed_file.insert_at(after_line, content);

        Ok(())
    }

    /// Show the current window (tab-separated line numbers).
    pub fn view(&self) -> String {
        self.windowed_file.format_window()
    }

    /// Jump to a 1-indexed line.
    pub fn goto(&mut self, line: usize) {
        self.windowed_file.goto(line);
    }

    /// Undo the last edit.
    pub fn undo_edit(&mut self) {
        if let Some(snapshot) = self.edit_history.pop() {
            self.windowed_file.restore(snapshot);
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ───────────────────────────────────────────────────────────────

    /// Build a WindowedFile with `n_lines` of content "0\n1\n2\n..." and
    /// window size 10, mirroring the TS test setup.
    fn make_wf(n_lines: usize, window_size: usize) -> WindowedFile {
        let content: Vec<String> = (0..n_lines).map(|i| i.to_string()).collect();
        let mut wf = WindowedFile::new(window_size);
        wf.set_content(&content.join("\n"));
        wf
    }

    // ── basic operations ──────────────────────────────────────────────────────

    #[test]
    fn init_properties() {
        let wf = make_wf(100, 10);
        assert_eq!(wf.first_line(), 0);
        assert_eq!(wf.window(), 10);
        assert_eq!(wf.line_count(), 100);
    }

    #[test]
    fn line_range_correct() {
        let mut wf = make_wf(100, 10);
        wf.set_first_line(10);
        let (start, end) = wf.line_range();
        assert_eq!(end - start, wf.window() - 1);
        assert_eq!((start, end), (10, 19));
    }

    #[test]
    fn get_window_text_contains_lines() {
        let mut wf = make_wf(100, 10);
        wf.set_first_line(10);
        let text = wf.get_window_text(None, false, false);
        // lines 10..=19 (0-indexed) → content "10" through "19"
        assert!(text.contains("10"));
        assert!(text.contains("19"));
    }

    // ── navigation: goto (center mode) ───────────────────────────────────────

    #[test]
    fn goto_center_clamps_at_zero() {
        let mut wf = make_wf(100, 10);
        wf.goto(1); // line 1 (1-indexed) → 0-indexed = 0; 0 - 5 = 0
        assert_eq!(wf.first_line(), 0);
    }

    #[test]
    fn goto_center_mid_file() {
        let mut wf = make_wf(100, 10);
        // TS goto(50, 'center'): firstLine = max(0, 50 - floor(10/2)) = max(0, 45) = 45
        // In our 1-indexed API: goto(51) → line0=50, half=5, start=45
        wf.goto(51);
        assert_eq!(wf.first_line(), 45);
    }

    #[test]
    fn goto_center_near_end() {
        let mut wf = make_wf(100, 10);
        // goto(100) → line0=99, half=5, start=94; end=min(103,99)=99 ✓
        wf.goto(100);
        assert_eq!(wf.first_line(), 94);
        assert_eq!(wf.line_range().1, 99);
    }

    // ── navigation: goto (top mode) ───────────────────────────────────────────

    #[test]
    fn goto_top_clamps_at_zero() {
        let mut wf = make_wf(100, 10);
        // TS: goto(0, 'top') → firstLine = max(0, 0 - floor(10*0.25)) = max(0,-2) = 0
        // Our 1-indexed: goto_top(1) → line0=0, offset=2, start=0
        wf.goto_top(1);
        assert_eq!(wf.first_line(), 0);
    }

    #[test]
    fn goto_top_mid_file() {
        let mut wf = make_wf(100, 10);
        // TS goto(50,'top'): firstLine = max(0, 50 - floor(10/4)) = max(0, 50-2) = 48
        // Our 1-indexed: goto_top(51) → line0=50, offset=2, start=48
        wf.goto_top(51);
        assert_eq!(wf.first_line(), 48);
    }

    #[test]
    fn goto_top_near_end_clamps() {
        let mut wf = make_wf(100, 10);
        // TS goto(100,'top'): firstLine = max(0,100-2)=98; end=min(107,99)=99 ✓
        // Our 1-indexed: goto_top(101) → line0=100 (out of range) but clamped
        // Let's use goto_top(100): line0=99, offset=2, start=97; end=min(106,99)=99
        wf.goto_top(100);
        assert_eq!(wf.line_range().1, 99);
    }

    // ── navigation: scroll ────────────────────────────────────────────────────

    #[test]
    fn scroll_positive() {
        let mut wf = make_wf(100, 10);
        wf.set_first_line(10);
        wf.scroll(10);
        assert_eq!(wf.first_line(), 20);
    }

    #[test]
    fn scroll_negative() {
        let mut wf = make_wf(100, 10);
        wf.set_first_line(20);
        wf.scroll(-10);
        assert_eq!(wf.first_line(), 10);
    }

    #[test]
    fn scroll_clamps_at_bottom() {
        let mut wf = make_wf(100, 10);
        wf.set_first_line(10);
        wf.scroll(-100);
        assert_eq!(wf.first_line(), 0);
    }

    #[test]
    fn scroll_clamps_at_top() {
        let mut wf = make_wf(100, 10);
        wf.set_first_line(10);
        wf.scroll(100);
        assert_eq!(wf.line_range().1, 99);
    }

    #[test]
    fn scroll_up_and_down_helpers() {
        let mut wf = make_wf(100, 10);
        wf.set_first_line(50);
        wf.scroll_down(); // +5
        assert_eq!(wf.first_line(), 55);
        wf.scroll_up(); // -5
        assert_eq!(wf.first_line(), 50);
    }

    // ── window output ─────────────────────────────────────────────────────────

    #[test]
    fn window_output_with_header_and_footer() {
        let mut wf = make_wf(100, 10);
        wf.set_first_line(10);
        let output = wf.get_window_text(Some("/tmp/test.py"), true, true);
        assert!(output.contains("[File: /tmp/test.py (100 lines total)]"));
        assert!(output.contains("(10 more lines above)"));
        // 0-indexed line 10 → 1-indexed "11", content = "10"
        assert!(output.contains("11:10"));
        // 0-indexed line 19 → 1-indexed "20", content = "19"
        assert!(output.contains("20:19"));
        assert!(output.contains("(80 more lines below)"));
    }

    #[test]
    fn single_line_file() {
        let mut wf = WindowedFile::new(10);
        wf.set_content("\n");
        assert_eq!(wf.line_count(), 1);
        assert_eq!(wf.line_range(), (0, 0));
        let output = wf.get_window_text(Some("/tmp/new.py"), true, true);
        assert!(output.contains("[File: /tmp/new.py (1 lines total)]"));
        assert!(output.contains("1:"));
    }

    #[test]
    fn format_window_tab_separated() {
        let mut wf = WindowedFile::new(5);
        wf.set_content("alpha\nbeta\ngamma");
        let out = wf.format_window();
        assert!(out.contains("1\talpha"));
        assert!(out.contains("2\tbeta"));
        assert!(out.contains("3\tgamma"));
    }

    // ── search ────────────────────────────────────────────────────────────────

    #[test]
    fn search_case_insensitive() {
        let mut wf = WindowedFile::new(10);
        wf.set_content("Hello World\nfoo\nhello rust\nbar");
        let results = wf.search("hello");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 1);
        assert_eq!(results[1].0, 3);
    }

    #[test]
    fn find_all_occurrences_one_based() {
        let mut wf = WindowedFile::new(10);
        wf.set_content("test\ntest\nother\ntest");
        let occ = wf.find_all_occurrences("test", false);
        assert_eq!(occ, vec![1, 2, 4]);
    }

    #[test]
    fn find_all_occurrences_zero_based() {
        let mut wf = WindowedFile::new(10);
        wf.set_content("test\ntest\nother\ntest");
        let occ = wf.find_all_occurrences("test", true);
        assert_eq!(occ, vec![0, 1, 3]);
    }

    // ── text replacement ──────────────────────────────────────────────────────

    #[test]
    fn replace_in_window() {
        let mut wf = make_wf(100, 10);
        wf.set_first_line(10);
        wf.replace_in_window("10", "Hello, world!").unwrap();
        assert_eq!(wf.line_count(), 100);
        // After replacement, offset back by floor(10*0.25)+1 = 3
        // window_start was 10, now 10 - 3 = 7
        assert_eq!(wf.line_range(), (7, 16));
        let text = wf.get_window_text(None, false, false);
        assert!(text.contains("Hello, world!"));
    }

    #[test]
    fn replace_in_window_error_not_found() {
        let mut wf = make_wf(100, 10);
        let result = wf.replace_in_window("asdf", "Hello, world!");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Text not found"));
    }

    #[test]
    fn global_replace() {
        let mut wf = WindowedFile::new(10);
        wf.set_content("old\nold\nother\nold");
        let (n, first) = wf.replace("old", "new");
        assert_eq!(n, 3);
        assert_eq!(first, Some(1));
        assert_eq!(wf.get_content(), "new\nnew\nother\nnew");
    }

    #[test]
    fn global_replace_no_match() {
        let mut wf = WindowedFile::new(10);
        wf.set_content("foo\nbar");
        let (n, first) = wf.replace("baz", "qux");
        assert_eq!(n, 0);
        assert_eq!(first, None);
    }

    // ── insert ────────────────────────────────────────────────────────────────

    #[test]
    fn insert_at_position() {
        let mut wf = WindowedFile::new(10);
        wf.set_content("line1\nline2\nline3");
        // Insert "inserted" at index 1 (after "line1"), mirrors TS splice(1,0,"inserted")
        let n = wf.insert_at(1, "inserted");
        assert_eq!(n, 1);
        assert_eq!(wf.get_content(), "line1\ninserted\nline2\nline3");
    }

    // ── StrReplaceEditor ──────────────────────────────────────────────────────

    #[test]
    fn str_replace_basic() {
        let mut ed = StrReplaceEditor::new(10);
        ed.set_content("hello world\nfoo bar");
        ed.str_replace("hello world", "goodbye world").unwrap();
        assert_eq!(ed.get_content(), "goodbye world\nfoo bar");
    }

    #[test]
    fn str_replace_not_found_error() {
        let mut ed = StrReplaceEditor::new(10);
        ed.set_content("hello world");
        let result = ed.str_replace("missing", "replacement");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn str_replace_multiple_occurrences_error() {
        let mut ed = StrReplaceEditor::new(10);
        ed.set_content("foo\nfoo\nbar");
        let result = ed.str_replace("foo", "baz");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("appears"));
    }

    #[test]
    fn str_replace_undo() {
        let mut ed = StrReplaceEditor::new(10);
        ed.set_content("original content");
        ed.str_replace("original content", "modified content").unwrap();
        assert_eq!(ed.get_content(), "modified content");
        ed.undo_edit();
        assert_eq!(ed.get_content(), "original content");
    }

    #[test]
    fn editor_insert_after_line() {
        let mut ed = StrReplaceEditor::new(10);
        ed.set_content("line1\nline2\nline3");
        ed.insert(1, "inserted").unwrap();
        assert_eq!(ed.get_content(), "line1\ninserted\nline2\nline3");
    }

    #[test]
    fn editor_insert_out_of_range() {
        let mut ed = StrReplaceEditor::new(10);
        ed.set_content("line1");
        let result = ed.insert(5, "bad");
        assert!(result.is_err());
    }

    #[test]
    fn editor_view_tab_separated() {
        let mut ed = StrReplaceEditor::new(5);
        ed.set_content("alpha\nbeta\ngamma");
        let view = ed.view();
        assert!(view.contains("1\talpha"));
        assert!(view.contains("2\tbeta"));
    }

    #[test]
    fn editor_goto_moves_window() {
        let mut ed = StrReplaceEditor::new(10);
        let content: Vec<String> = (1..=50).map(|i| i.to_string()).collect();
        ed.set_content(&content.join("\n"));
        ed.goto(30);
        // window_start = 30 - 1 - 5 = 24 (0-indexed); current_line() = window_start + 1 = 25
        assert_eq!(ed.windowed_file.current_line(), 25);
    }

    #[test]
    fn window_never_exceeds_file_bounds() {
        let mut wf = WindowedFile::new(10);
        wf.set_content("only one line");
        assert_eq!(wf.line_count(), 1);
        assert_eq!(wf.line_range(), (0, 0));
        wf.scroll(100);
        assert_eq!(wf.first_line(), 0);
    }
}
