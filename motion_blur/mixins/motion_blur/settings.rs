// Motion Blur's settings page.
fn motion_blur_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use horizon_client_render::native_motion_blur as m;
    if mod_id != m::ID {
        return;
    }
    let game = &*game;
    buttons.extend(m::settings(|key| game.native_setting(m::ID, key)).labels());
}
