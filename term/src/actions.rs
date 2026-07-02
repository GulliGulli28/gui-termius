// Vendored from iced_term (MIT, Copyright (c) 2024 Ilia Shvyrialkin) — see ../LICENSE-iced_term.
// Unmodified except where noted.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Action {
    Shutdown,
    ChangeTitle(String),
    #[default]
    Ignore,
}
