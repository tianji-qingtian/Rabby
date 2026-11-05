use gpui_kit::*;
use rabby_terminal_view::TerminalPane;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

pub enum PaneNode {
    Leaf(Entity<TerminalPane>),
    Split {
        direction: SplitDirection,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

impl PaneNode {
    pub fn render(&self) -> AnyElement {
        match self {
            PaneNode::Leaf(pane) => pane.clone().into_any_element(),
            PaneNode::Split {
                direction,
                first,
                second,
            } => {
                let mut container = div().flex().size_full();
                let divider = match direction {
                    SplitDirection::Horizontal => {
                        div().w(px(1.0)).h_full().flex_shrink_0().bg(rgb(0x2a2a33))
                    }
                    SplitDirection::Vertical => {
                        div().h(px(1.0)).w_full().flex_shrink_0().bg(rgb(0x2a2a33))
                    }
                };
                container = match direction {
                    SplitDirection::Horizontal => container.flex_row(),
                    SplitDirection::Vertical => container.flex_col(),
                };
                container
                    .child(div().flex_1().overflow_hidden().child(first.render()))
                    .child(divider)
                    .child(div().flex_1().overflow_hidden().child(second.render()))
                    .into_any_element()
            }
        }
    }

    pub fn find(&self, id: EntityId) -> Option<Entity<TerminalPane>> {
        match self {
            PaneNode::Leaf(pane) if pane.entity_id() == id => Some(pane.clone()),
            PaneNode::Leaf(_) => None,
            PaneNode::Split { first, second, .. } => first.find(id).or_else(|| second.find(id)),
        }
    }

    pub fn split_active(
        &mut self,
        active: EntityId,
        new_pane: Entity<TerminalPane>,
        direction: SplitDirection,
    ) -> bool {
        match self {
            PaneNode::Leaf(pane) if pane.entity_id() == active => {
                let old = pane.clone();
                *self = PaneNode::Split {
                    direction,
                    first: Box::new(PaneNode::Leaf(old)),
                    second: Box::new(PaneNode::Leaf(new_pane)),
                };
                true
            }
            PaneNode::Split { first, second, .. } => {
                first.split_active(active, new_pane.clone(), direction)
                    || second.split_active(active, new_pane, direction)
            }
            _ => false,
        }
    }
}
