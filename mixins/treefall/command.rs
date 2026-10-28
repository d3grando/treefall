// commands::Game::run_command HEAD: handle `/treefall`, pass everything else on.
fn treefall_command(game: &mut crate::game::Game<'_>, command: &str) -> core::ops::ControlFlow<bool> {
    match game.treefall_command(command) {
        Some(result) => core::ops::ControlFlow::Break(result),
        None => core::ops::ControlFlow::Continue(()),
    }
}
