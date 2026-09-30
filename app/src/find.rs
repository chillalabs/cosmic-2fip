//! Find Files (Ctrl+F): a dialog that searches the active panel's folder and
//! its subfolders by name, and opens the chosen result in the panel.

use std::path::{Path, PathBuf};

use cosmic::iced::widget::scrollable::{snap_to, RelativeOffset};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;
use fs_ops::ops::CancelHandle;
use fs_ops::search::SearchHit;

use crate::app::Message;
use crate::fl;

/// A search stops after this many results; a pattern that broad needs
/// narrowing anyway.
pub const RESULT_LIMIT: usize = 1000;

const DIALOG_WIDTH: f32 = 680.0;
const RESULTS_HEIGHT: f32 = 360.0;
const ROW_HEIGHT: f32 = 40.0;
const NAME_SIZE: f32 = 13.0;
const PATH_SIZE: f32 = 11.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindStatus {
    /// Nothing searched yet.
    Ready,
    Searching,
    Done {
        truncated: bool,
    },
}

pub struct FindState {
    /// The folder searched (the active panel's, when the dialog opened).
    pub root: PathBuf,
    pub pattern: String,
    pub hits: Vec<SearchHit>,
    pub status: FindStatus,
    /// Stops the running search.
    pub cancel: CancelHandle,
    /// Tells results of the current search from late ones of a previous one.
    pub generation: u64,
    /// The highlighted result (↑/↓, click); Enter or double-click opens it.
    pub cursor: Option<usize>,
    /// Whether the keyboard is in the name field (else in the results).
    pub input_focused: bool,
    pub input_id: widget::Id,
    pub results_id: widget::Id,
}

impl FindState {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            pattern: String::new(),
            hits: Vec::new(),
            status: FindStatus::Ready,
            cancel: CancelHandle::new(),
            generation: 0,
            cursor: None,
            input_focused: true,
            input_id: widget::Id::unique(),
            results_id: widget::Id::unique(),
        }
    }

    /// Moves the highlight `step` rows (clamped to the list), and scrolls
    /// the results so it stays visible.
    pub fn move_cursor(&mut self, step: isize) -> cosmic::app::Task<Message> {
        let Some(last) = self.hits.len().checked_sub(1) else {
            return cosmic::app::Task::none();
        };
        let current = self.cursor.unwrap_or(0) as isize;
        let target = (current + step).clamp(0, last as isize) as usize;
        self.cursor = Some(target);
        self.scroll_to_cursor()
    }

    fn scroll_to_cursor(&self) -> cosmic::app::Task<Message> {
        let (Some(cursor), count) = (self.cursor, self.hits.len()) else {
            return cosmic::app::Task::none();
        };
        // Same trick as the file list: snapping to the row's relative
        // position keeps it on screen.
        let y = if count > 1 {
            cursor as f32 / (count - 1) as f32
        } else {
            0.0
        };
        snap_to(
            self.results_id.clone(),
            RelativeOffset {
                x: None,
                y: Some(y),
            },
        )
    }
}

/// How many rows PageUp / PageDown move.
pub fn page_rows() -> isize {
    (RESULTS_HEIGHT / ROW_HEIGHT) as isize - 1
}

/// The Find Files dialog.
pub fn view(find: &FindState) -> Element<'_, Message> {
    let searching = find.status == FindStatus::Searching;
    let search_button = if searching {
        widget::button::standard(fl!("find-stop")).on_press(Message::FindStop)
    } else {
        widget::button::suggested(fl!("find-search"))
            .on_press_maybe((!find.pattern.trim().is_empty()).then_some(Message::FindStart))
    };
    let input_row = widget::Row::new()
        .spacing(8)
        .align_y(Alignment::Center)
        .push(
            widget::text_input(fl!("find-placeholder"), find.pattern.as_str())
                .id(find.input_id.clone())
                .on_input(Message::FindPatternChanged)
                .on_submit(|_| Message::FindStart)
                .size(NAME_SIZE)
                .width(Length::Fill),
        )
        .push(search_button);

    let count = find.hits.len();
    let status = match find.status {
        FindStatus::Ready => fl!("find-hint"),
        FindStatus::Searching => fl!("find-searching", count = count),
        FindStatus::Done { .. } if count == 0 => fl!("find-no-results"),
        FindStatus::Done { truncated: true } => fl!("find-truncated", count = count),
        FindStatus::Done { truncated: false } => fl!("find-results", count = count),
    };

    let mut rows = widget::Column::new().spacing(0);
    for (index, hit) in find.hits.iter().enumerate() {
        rows = rows.push(result_row(
            &find.root,
            hit,
            index,
            find.cursor == Some(index),
        ));
    }

    let content = widget::Column::new()
        .spacing(12)
        .push(widget::text::title4(fl!("find")))
        .push(widget::text(fl!("find-in", path = find.root.display().to_string())).size(PATH_SIZE))
        .push(input_row)
        .push(widget::text::caption(status))
        .push(
            widget::container(
                widget::scrollable(rows)
                    .id(find.results_id.clone())
                    .height(Length::Fill),
            )
            .height(Length::Fixed(RESULTS_HEIGHT)),
        );

    widget::dialog()
        .width(Length::Fixed(DIALOG_WIDTH))
        .control(content)
        .primary_action(
            widget::button::suggested(fl!("find-go-to"))
                .on_press_maybe(find.cursor.map(Message::FindGoTo)),
        )
        .secondary_action(widget::button::standard(fl!("close")).on_press(Message::FindClose))
        .into()
}

/// One result: its icon and name, and below it the folder it's in
/// (relative to the searched folder).
fn result_row<'a>(
    root: &Path,
    hit: &'a SearchHit,
    index: usize,
    highlighted: bool,
) -> Element<'a, Message> {
    let icon = if hit.is_dir {
        "folder-symbolic"
    } else {
        "text-x-generic-symbolic"
    };
    let name = hit
        .path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let folder = hit.path.parent().unwrap_or(root);
    let folder = match folder.strip_prefix(root) {
        Ok(relative) if relative.as_os_str().is_empty() => root.display().to_string(),
        Ok(relative) => relative.display().to_string(),
        Err(_) => folder.display().to_string(),
    };
    let row = widget::Row::new()
        .spacing(8)
        .align_y(Alignment::Center)
        .push(widget::icon::from_name(icon).size(16))
        .push(
            widget::Column::new()
                .push(widget::text(name).size(NAME_SIZE))
                .push(widget::text(folder).size(PATH_SIZE)),
        );
    widget::mouse_area(
        widget::container(row)
            .padding([0, 8])
            .height(Length::Fixed(ROW_HEIGHT))
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .class(cosmic::theme::Container::custom(move |theme| {
                crate::app::highlight_style(theme, highlighted)
            })),
    )
    .on_press(Message::FindHighlight(index))
    .on_double_click(Message::FindGoTo(index))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hits(count: usize) -> Vec<SearchHit> {
        (0..count)
            .map(|i| SearchHit {
                path: PathBuf::from(format!("/r/{i}")),
                is_dir: false,
            })
            .collect()
    }

    #[test]
    fn cursor_moves_within_the_results() {
        let mut find = FindState::new(PathBuf::from("/r"));
        let _ = find.move_cursor(1);
        assert_eq!(find.cursor, None, "no results, no highlight");

        find.hits = hits(3);
        let _ = find.move_cursor(1);
        assert_eq!(find.cursor, Some(1));
        let _ = find.move_cursor(10);
        assert_eq!(find.cursor, Some(2));
        let _ = find.move_cursor(-10);
        assert_eq!(find.cursor, Some(0));
    }
}
