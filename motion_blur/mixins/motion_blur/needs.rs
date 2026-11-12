// The blur reads the world's depth: keep it while the mod is on.
fn motion_blur_needs(needs: &mut crate::gfx::native_render::RenderNeeds) {
    crate::native_motion_blur::needs(&mut needs.world_depth);
}
