use bevy::prelude::*;

struct DesktopPlugin;

impl Plugin for DesktopPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, open_file_picker_requests);
    }
}

fn open_file_picker_requests(
    mut activations: MessageReader<app::platform::UiFilePickerActivated>,
    pickers: Query<(
        Has<app::platform::UiDirectoryPicker>,
        Has<app::platform::UiAudioFilePicker>,
    )>,
    mut results: MessageWriter<app::platform::UiFilePickerResult>,
) {
    for activation in activations.read() {
        let Ok((directory, audio_file)) = pickers.get(activation.picker) else {
            continue;
        };

        let dialog = rfd::FileDialog::new();
        let path = if directory {
            dialog.set_title("Choose ROM directory").pick_folder()
        } else if audio_file {
            dialog
                .set_title("Choose WAV sample (*.wav)")
                .add_filter("WAV audio", &["wav"])
                .pick_file()
        } else {
            dialog
                .set_title("Choose GameBoy ROM (*.gb, *.gbc)")
                .add_filter("GameBoy ROM", &["gb", "gbc"])
                .pick_file()
        };
        let Some(path) = path else {
            continue;
        };

        let path = path.canonicalize().unwrap_or(path);
        results.write(app::platform::UiFilePickerResult {
            picker: activation.picker,
            value: path.display().to_string(),
        });
    }
}

fn main() {
    app::run_app(DesktopPlugin);
}
