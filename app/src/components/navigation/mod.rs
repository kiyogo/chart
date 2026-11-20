pub mod conversations;
pub mod modals;
pub mod rail;
pub mod sidebar;

#[derive(Clone, PartialEq, Eq)]
pub enum NavSection {
    DirectMessages,
    Friends,
    Communities,
    Server(String),
    Meet,
    Feed,
    VicoAi,
    Search,
    Download,
    Settings,
}

