// The world is drawn, the hand and HUD not yet: blur it along the camera's motion.
fn motion_blur_world_drawn(world: &mut crate::gfx::native_render::WorldDrawn<'_>) {
    crate::native_motion_blur::draw(
        world.device,
        world.queue,
        world.encoder,
        world.color,
        world.color_view,
        world.color_format,
        world.size,
        world.depth,
        world.depth_size,
        world.view_proj,
        world.camera,
    );
}
