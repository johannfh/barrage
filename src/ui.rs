use std::collections::{HashMap, hash_map::Entry};

use bevy::prelude::*;

#[derive(Debug, Clone)]
pub enum CommandPayload {
    None,
    TargetPoint(Vec2),
    TargetEntity(Entity),
}

#[derive(Debug, Clone)]
pub struct CommandEvent {
    pub command_type: String,
    pub payload: CommandPayload,
}

/// This tells the input system how to handle user input for a specific command.
/// Some commands require targeting (e.g., attack command needs a target entity),
/// while others can be executed immediately (e.g., stop command).
/// This is a polymorphic behavior that can be extended for different command types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandInputMode {
    /// Command is executed immediately without targeting.
    /// Results in `CommandPayload::None`.
    /// Examples include stop or hold position commands.
    Immediate,
    /// Command requires a spatial target (e.g., point on the map).
    /// Results in `CommandPayload::TargetPoint`.
    /// Examples include right-click move commands.
    ImmediateSpatial,
    /// Command requires a target point on the map.
    /// Results in `CommandPayload::TargetPoint`.
    /// Examples include move-to-point commands.
    SelectTargetedPoint,
    /// Command requires a target entity.
    /// Results in `CommandPayload::TargetEntity`.
    /// Examples include interact-with-entity commands, e.g. special abilities.
    SelectTargetedEntity,
    /// Command requires selecting either a point or an entity.
    /// Results in either `CommandPayload::TargetPoint` or `CommandPayload::TargetEntity`.
    /// Examples include context-sensitive commands that can target both, like attack-move.
    SelectTargetedPointOrEntity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PanelTransition {
    Push(String),
    Pop,
}

#[derive(Debug, Clone)]
struct CommandEntry {
    command_type: String,
    input_mode: CommandInputMode,
}

#[derive(Resource, Default)]
pub struct CommandRegistry {
    behaviors: HashMap<String, CommandEntry>,
}

impl CommandRegistry {
    /// Registers a new command behavior.
    /// If a behavior for the same command type already exists,
    /// it will be overwritten, but a warning will be logged.
    fn register(&mut self, behavior: CommandEntry) {
        match self.behaviors.entry(behavior.command_type.clone()) {
            Entry::Vacant(e) => {
                e.insert(behavior);
            }
            Entry::Occupied(mut e) => {
                warn!(
                    "Existing ommand behavior for '{}' will be overwritten: {:?} -> {:?}",
                    behavior.command_type,
                    e.get(),
                    behavior
                );
                e.insert(behavior);
            }
        }
    }

    fn get(&self, command_type: &str) -> Option<&CommandEntry> {
        self.behaviors.get(command_type)
    }
}

/// Action associated with a control panel entry.
#[derive(Debug, Clone)]
enum ControlPanelAction {
    /// Execute a command identified by its command ID.
    ExecuteCommand(String),
    /// Transition to another control panel state.
    TransitionPanel(PanelTransition),
    ExecuteAndTransition {
        command_id: String,
        transition: PanelTransition,
    },
}

/// Control panel layout for entities.
#[derive(Debug, Clone, Default)]
struct ControlPanel {
    /// 5x3 grid for commands. Each entry can be `Some(command_id)` or `None` for empty slots.
    /// To execute a command, the UI system will look up the command ID in the [`CommandRegistry`].
    entries: [[Option<ControlPanelAction>; 5]; 3],
}

/// Control panel tree for different entity states.
struct ControlPanelTree {
    /// Root panel identifier.
    root: String,
    /// Control panels for different states, identified by state name.
    panels: HashMap<String, ControlPanel>,
}

/// Control panel registry for entity types.
#[derive(Resource, Default)]
pub struct ControlPanelRegistry {
    /// Control panel trees for different entity types, identified by entity type name.
    /// Each tree contains panels for various states of that entity type.
    /// This allows for dynamic navigation between different control panels based on the entity's
    /// state, e.g. when a worker is selected, the panel might switch between "root" and "building"
    /// states. This structure supports complex UI interactions in the control panel.
    panels: HashMap<String, ControlPanelTree>,
}

impl ControlPanelRegistry {
    /// Registers a control panel tree for a specific entity type.
    /// If a panel tree for the same entity type already exists,
    /// it will be overwritten, but a warning will be logged.
    fn register(&mut self, entity_type: String, panel_tree: ControlPanelTree) {
        match self.panels.entry(entity_type.clone()) {
            Entry::Vacant(e) => {
                e.insert(panel_tree);
            }
            Entry::Occupied(mut e) => {
                warn!(
                    "Existing control panel tree for '{}' will be overwritten.",
                    entity_type
                );
                e.insert(panel_tree);
            }
        }
    }

    fn get(&self, entity_type: &str) -> Option<&ControlPanelTree> {
        self.panels.get(entity_type)
    }
}

pub trait CommandDispatcher: std::fmt::Debug + Send + Sync + 'static {
    fn catches(&self, command_type: &str) -> bool;
    fn dispatch_command(&self, command_event: CommandEvent);
}

macro_rules! impl_command_dispatcher {
    (
        catches: [$($cmd_type:expr),*],
        dispatcher: $dispatcher_fn:expr
    ) => {
        {
            struct DispatcherImpl;

            impl CommandDispatcher for DispatcherImpl {
                fn catches(&self, command_type: &str) -> bool {
                    match command_type {
                        $(
                            $cmd_type => true,
                        )*
                        _ => false,
                    }
                }

                fn dispatch_command(&self, command_event: CommandEvent) {
                    ($dispatcher_fn)(command_event);
                }
            }

            impl std::fmt::Debug for DispatcherImpl {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "DispatcherImpl {{ catches: [{:?}] }}", vec![$($cmd_type),*])
                }
            }

            Box::new(DispatcherImpl) as Box<dyn CommandDispatcher>
        }
    };
}

#[derive(Resource, Default)]
struct CommandDispatcherPipeline {
    dispatchers: Vec<Box<dyn CommandDispatcher>>,
}

impl CommandDispatcherPipeline {
    fn dispatch(&self, command_event: CommandEvent) {
        for dispatcher in &self.dispatchers {
            if dispatcher.catches(&command_event.command_type) {
                dispatcher.dispatch_command(command_event.clone());
            }
        }
    }

    fn register_dispatcher(&mut self, dispatcher: Box<dyn CommandDispatcher>) {
        info!("Registering command dispatcher: {:?}", dispatcher);
        self.dispatchers.push(dispatcher);
    }
}

fn setup_ui(
    mut commands: Commands,
    mut command_registry: ResMut<CommandRegistry>,
    mut control_panel_registry: ResMut<ControlPanelRegistry>,
    mut dispatcher_pipeline: ResMut<CommandDispatcherPipeline>,
) {
    const WORKER_ENTITY_TYPE: &str = "core:worker";
    const MOVE_COMMAND_ID: &str = "core:move";

    command_registry.register(CommandEntry {
        command_type: "core:move".to_string(),
        input_mode: CommandInputMode::SelectTargetedPoint,
    });
    control_panel_registry.register(
        WORKER_ENTITY_TYPE.to_string(),
        ControlPanelTree {
            root: "root".to_string(),
            panels: {
                let move_action = ControlPanelAction::ExecuteAndTransition {
                    command_id: MOVE_COMMAND_ID.to_string(),
                    transition: PanelTransition::Push("/build".to_string()),
                };
                let root_panel = ControlPanel {
                    entries: [
                        [Some(move_action), None, None, None, None],
                        [None, None, None, None, None],
                        [None, None, None, None, None],
                    ],
                };
                let cancel_action = ControlPanelAction::ExecuteAndTransition {
                    command_id: "core:cancel".to_string(),
                    transition: PanelTransition::Pop,
                };
                let build_panel = ControlPanel {
                    entries: [
                        [None, None, None, None, Some(cancel_action)],
                        [None, None, None, None, None],
                        [None, None, None, None, None],
                    ],
                };
                let mut panels = HashMap::new();
                panels.insert("/".to_string(), root_panel);
                panels.insert("/build".to_string(), build_panel);
                panels
            },
        },
    );

    let move_dispatcher = impl_command_dispatcher!(
        catches: ["core:move"],
        dispatcher: |event: CommandEvent| {
            info!("Dispatching move command: {:?}", event);
        }
    );
    dispatcher_pipeline.register_dispatcher(move_dispatcher);
}

pub struct UserControlsPlugin;

impl Plugin for UserControlsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CommandRegistry>()
            .init_resource::<ControlPanelRegistry>()
            .init_resource::<CommandDispatcherPipeline>()
            .add_systems(Startup, setup_ui);
    }
}
