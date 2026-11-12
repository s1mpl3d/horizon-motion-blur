// Motion Blur's row in Options > Mods.
fn motion_blur_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use horizon_client_render::native_motion_blur as m;
    mods.push(crate::game::native_hooks::NativeMod {
        id: m::ID.into(),
        title: format!("{} {}", m::NAME, m::VERSION),
        description: m::DESCRIPTION.into(),
        keys: vec![],
    });
}
