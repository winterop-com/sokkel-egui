//! Every board, as the web's `pages/Boards.tsx` lists them: in the person's order, dragged by their
//! grips, the order kept in the preferences.

use crate::backend::Backend;
use crate::model::{Board, counted};

/// The header's count.
pub fn count(boards: &[Board]) -> String {
    counted(boards.len(), "board")
}

/// The boards page's own state: the order as last dragged, until the backend tells it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoardsPage {
    pub order: Option<Vec<Board>>,
}

impl BoardsPage {
    /// The boards shown: the order last dragged while it holds the same boards, else the backend's.
    pub fn shown(&self, boards: &[Board]) -> Vec<Board> {
        match &self.order {
            Some(order) if order.len() == boards.len() && order.iter().all(|board| boards.contains(board)) => {
                order.clone()
            }
            _ => boards.to_vec(),
        }
    }

    /// A board dragged from one place to another.
    pub fn reorder(&mut self, backend: &mut Backend, boards: &[Board], prefs: &str, from: usize, to: usize) {
        let shown = self.shown(boards);
        self.order = Some(backend.reorder_boards(&shown, from, to, prefs));
    }
}
