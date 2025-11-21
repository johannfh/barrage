use bevy::prelude::*;

#[derive(Message)]
pub struct ToastMessage {
    pub content: String,
}

// for now, just print to console
fn show_toast_system(mut toast_reader: MessageReader<ToastMessage>) {
    for toast in toast_reader.read() {
        println!("TOAST: {}", toast.content);
    }
}

pub struct ToastsPlugin;

impl Plugin for ToastsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ToastMessage>()
            .add_systems(Update, show_toast_system);
    }
}
