// Each frame, on the game thread: tell the renderer whether the blur is on, its settings and the
// frame's length.
fn motion_blur_frame(game: &mut crate::game::Game<'_>, elapsed: std::time::Duration) {
    use horizon_client_render::native_motion_blur as m;
    let game = &*game;
    m::publish(game.native_on(m::ID), elapsed.as_secs_f32(), |key| {
        game.native_setting(m::ID, key)
    });
}
