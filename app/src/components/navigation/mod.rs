pub mod modals;
pub mod rail;
pub mod sidebar;

#[derive(Clone, PartialEq, Eq)]
pub enum NavSection {
    DirectMessages,
    Friends,
    Server(String),
    Settings,
}

