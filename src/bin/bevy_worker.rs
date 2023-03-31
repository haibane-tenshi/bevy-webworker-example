use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;
use bevy::window::WebElement;
use web_sys::OffscreenCanvas;

/// Query primary window and set up the handle to it so rendering can pick it up.
///
/// Normally this job is done by WinitPlugin, however it is hopelessly broken for web workers.
/// We definitely don't do everything that we need to, but this is enough to get us rendering.
///
/// Notably it doesn't properly communicate viewport size to bevy.
/// Currently it works because both sides use hardcoded 1280x720.
#[derive(Default)]
pub struct RegisterPrimaryWindow;

impl Plugin for RegisterPrimaryWindow {
    fn build(&self, app: &mut App) {
        use bevy::ecs::system::SystemState;
        use bevy::window::{AbstractHandleWrapper, PrimaryWindow};

        #[allow(clippy::type_complexity)]
        let mut system_state: SystemState<(
            Commands,
            Query<(Entity, &Window, With<PrimaryWindow>)>,
        )> = SystemState::from_world(&mut app.world);
        let (mut commands, query) = system_state.get_mut(&mut app.world);

        let (entity, window, _) = query.get_single().unwrap();

        let handle: AbstractHandleWrapper = {
            use bevy::window::WebHandle;

            let web_handle = match &window.web_element {
                WebElement::OffscreenCanvas(canvas) => WebHandle::OffscreenCanvas(canvas.clone()),
                // Ignore other options.
                _ => unreachable!(),
            };

            AbstractHandleWrapper::WebHandle(web_handle)
        };

        commands.entity(entity).insert(handle);
        system_state.apply(&mut app.world);
    }
}

/// Refreshed version of Bevy's default plugins, now with web-worker flavor.
///
/// Note: it isn't a faithful recreation of `DefaultPlugins` with all configs, it just works here.
pub struct DefaultPlugins {
    primary_window: WebElement,
}

impl PluginGroup for DefaultPlugins {
    fn build(self) -> PluginGroupBuilder {
        use bevy::a11y::AccessibilityPlugin;
        use bevy::app::ScheduleRunnerPlugin;
        use bevy::core_pipeline::CorePipelinePlugin;
        use bevy::diagnostic::DiagnosticsPlugin;
        use bevy::input::InputPlugin;
        use bevy::log::LogPlugin;
        use bevy::render::RenderPlugin;
        use bevy::sprite::SpritePlugin;
        use bevy::time::TimePlugin;

        let window_plugin = {
            let primary_window = Window {
                web_element: self.primary_window,
                ..Window::default()
            };

            let primary_window = Some(primary_window);

            WindowPlugin {
                primary_window,
                ..WindowPlugin::default()
            }
        };

        PluginGroupBuilder::start::<Self>()
            .add(LogPlugin::default())
            .add(TaskPoolPlugin::default())
            .add(TypeRegistrationPlugin::default())
            .add(TimePlugin::default())
            .add(FrameCountPlugin::default())
            .add(TransformPlugin::default())
            .add(HierarchyPlugin::default())
            .add(DiagnosticsPlugin::default())
            .add(InputPlugin::default())
            .add(window_plugin)
            .add(AccessibilityPlugin)
            .add(RegisterPrimaryWindow::default())
            .add(AssetPlugin::default())
            .add(RenderPlugin::default())
            .add(ImagePlugin::default())
            .add(CorePipelinePlugin)
            .add(SpritePlugin::default())
            .add(ScheduleRunnerPlugin::default())
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    use bevy::sprite::MaterialMesh2dBundle;

    commands.spawn(Camera2dBundle::default());

    // Circle
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(shape::Circle::new(50.).into()).into(),
        material: materials.add(ColorMaterial::from(Color::PURPLE)),
        transform: Transform::from_translation(Vec3::new(-150., 0., 0.)),
        ..default()
    });

    // Rectangle
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(0.25, 0.25, 0.75),
            custom_size: Some(Vec2::new(50.0, 100.0)),
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(-50., 0., 0.)),
        ..default()
    });

    // Quad
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes
            .add(shape::Quad::new(Vec2::new(50., 100.)).into())
            .into(),
        material: materials.add(ColorMaterial::from(Color::LIME_GREEN)),
        transform: Transform::from_translation(Vec3::new(50., 0., 0.)),
        ..default()
    });

    // Hexagon
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(shape::RegularPolygon::new(50., 6).into()).into(),
        material: materials.add(ColorMaterial::from(Color::TURQUOISE)),
        transform: Transform::from_translation(Vec3::new(150., 0., 0.)),
        ..default()
    });
}

fn single_pass(canvas: OffscreenCanvas) {
    use bevy::app::ScheduleRunnerSettings;

    App::new()
        .insert_resource(ScheduleRunnerSettings::run_once())
        .add_plugins(DefaultPlugins {
            primary_window: WebElement::OffscreenCanvas(canvas),
        })
        .add_systems(Startup, setup)
        .run();
}

// Adapted from https://github.com/thedodd/trunk/blob/master/examples/webworker/src/bin/worker.rs
fn main() {
    use js_sys::Array;
    use wasm_bindgen::prelude::{Closure, JsCast, JsValue};
    use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

    let scope = DedicatedWorkerGlobalScope::from(JsValue::from(js_sys::global()));

    let onmessage = Closure::wrap(Box::new(move |msg: MessageEvent| {
        let offscreen_canvas = msg
            .data()
            .dyn_into::<OffscreenCanvas>()
            .expect("message must be an OffscreenCanvas");
        single_pass(offscreen_canvas);
    }) as Box<dyn Fn(MessageEvent)>);
    scope.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // The worker must send a message to indicate that it's ready to receive messages.
    scope
        .post_message(&Array::new().into())
        .expect("posting ready message succeeds");
}
