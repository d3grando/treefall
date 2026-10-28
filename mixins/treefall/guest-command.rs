// lan_actions::Game::guest_command HEAD: queue a LAN guest's `/treefall` so it
// runs as that guest (their own on/off state), whatever their permission level.
fn treefall_guest_command(game: &mut crate::game::Game<'_>, host: &mut crate::game::lan::LanHost, conn: u32, command: &str) -> core::ops::ControlFlow<()> {
    if !crate::game::treefall::is_command(command) {
        return core::ops::ControlFlow::Continue(());
    }
    if let Some(permission) = host.hub.guest(conn).map(|g| g.permission) {
        host.commands.push(crate::game::lan_actions::GuestCommand {
            conn, command: command.trim().trim_start_matches('/').to_owned(), permission,
            integrated: host.hub.config.integrated, quiet: false,
        });
    }
    let _ = game;
    core::ops::ControlFlow::Break(())
}
