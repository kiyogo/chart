pub mod modals;
pub mod sidebar;

#[derive(Clone, PartialEq, Eq)]
pub enum NavSection {
    DirectMessages,
    Friends,
    Communities,
    Server(String),
    Settings,
}

