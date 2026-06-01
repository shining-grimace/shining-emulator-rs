use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, Mutex};

use base64::Engine;
use bevy::{prelude::*, winit::WinitSettings};
use jni::errors::ThrowRuntimeExAndDefault;
use jni::objects::{Global, JObject, JString, JValue};
use jni::sys::{jint, jlong};
use jni::{EnvUnowned, JavaVM, jni_sig, jni_str};

struct MobilePlugin;

const PICKER_KIND_ROM: u32 = 0;
const PICKER_KIND_DIRECTORY: u32 = 1;
const PICKER_KIND_AUDIO: u32 = 2;

struct AndroidActivity {
    vm: JavaVM,
    activity: Global<JObject<'static>>,
}

#[derive(Clone, Debug)]
struct AndroidFilePickerResult {
    request_id: u64,
    value: String,
}

#[derive(Clone, Debug)]
struct AndroidTextInputChange {
    value: String,
    cursor_utf16: usize,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AndroidLocalDirectoryRomPayload {
    file_name: String,
    base64: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AndroidTextInputStateSnapshot {
    entity: Option<Entity>,
    value: String,
    cursor: usize,
}

#[derive(Debug)]
struct AndroidFilePickerRequests {
    next_request_id: u64,
    pending: HashMap<u64, Entity>,
}

impl Default for AndroidFilePickerRequests {
    fn default() -> Self {
        Self {
            next_request_id: 1,
            pending: HashMap::new(),
        }
    }
}

#[derive(Default, Resource)]
struct AndroidTextInputBridgeState {
    current: AndroidTextInputStateSnapshot,
    applied_from_android: Option<AndroidTextInputStateSnapshot>,
}

static ANDROID_ACTIVITY: LazyLock<Mutex<Option<AndroidActivity>>> =
    LazyLock::new(|| Mutex::new(None));

static ANDROID_FILE_PICKER_RESULTS: LazyLock<Mutex<VecDeque<AndroidFilePickerResult>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));

static ANDROID_TEXT_INPUT_CHANGES: LazyLock<Mutex<VecDeque<AndroidTextInputChange>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));

impl Plugin for MobilePlugin {
    // Sets 60 fps; note this can be changed at runtime if needed
    fn build(&self, app: &mut App) {
        app::platform::set_android_local_directory_reader(read_android_local_directory_roms);
        app.insert_resource(WinitSettings::mobile())
            .init_resource::<AndroidTextInputBridgeState>()
            .add_systems(
                Update,
                (
                    open_file_picker_requests,
                    apply_android_text_input_changes,
                    sync_android_text_input_keyboard,
                )
                    .chain(),
            );
    }
}

#[bevy_main]
fn main() {
    app::run_app(MobilePlugin);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_shiningemulator_MainActivity_nativeSetActivity<
    'local,
>(
    mut env: EnvUnowned<'local>,
    _activity: JObject<'local>,
    activity: JObject<'local>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        let vm = env.get_java_vm()?;
        let activity = env.new_global_ref(activity)?;
        let Ok(mut state) = ANDROID_ACTIVITY.lock() else {
            eprintln!("failed to lock Android activity state");
            return Ok(());
        };
        *state = Some(AndroidActivity { vm, activity });
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_shiningemulator_MainActivity_nativeOnFilePickerResult<
    'local,
>(
    mut env: EnvUnowned<'local>,
    _activity: JObject<'local>,
    request_id: jlong,
    uri: JString<'local>,
) {
    if uri.is_null() {
        return;
    }

    env.with_env(|env| -> jni::errors::Result<()> {
        let uri = uri.try_to_string(env)?;
        push_android_file_picker_result(request_id as u64, uri);
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_shiningemulator_MainActivity_nativeOnTextInputChanged<
    'local,
>(
    mut env: EnvUnowned<'local>,
    _activity: JObject<'local>,
    value: JString<'local>,
    cursor_utf16: jint,
) {
    if value.is_null() {
        return;
    }

    env.with_env(|env| -> jni::errors::Result<()> {
        let value = value.try_to_string(env)?;
        push_android_text_input_change(
            value,
            usize::try_from(cursor_utf16.max(0)).unwrap_or_default(),
        );
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

fn read_android_local_directory_roms(
    uri: &str,
) -> Result<Vec<app::platform::AndroidLocalDirectoryRomFile>, String> {
    let json = read_android_local_directory_roms_json(uri)?;
    let payloads = serde_json::from_str::<Vec<AndroidLocalDirectoryRomPayload>>(&json)
        .map_err(|error| format!("Directory response could not be parsed: {error}"))?;
    payloads
        .into_iter()
        .map(|payload| {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(payload.base64)
                .map_err(|error| format!("ROM file bytes could not be decoded: {error}"))?;
            Ok(app::platform::AndroidLocalDirectoryRomFile {
                file_name: payload.file_name,
                bytes,
            })
        })
        .collect()
}

fn read_android_local_directory_roms_json(uri: &str) -> Result<String, String> {
    let state = ANDROID_ACTIVITY
        .lock()
        .map_err(|_| "Android activity state could not be locked.".to_string())?;
    let Some(state) = state.as_ref() else {
        return Err("Android activity is not registered yet.".to_string());
    };

    state
        .vm
        .attach_current_thread(|env| -> jni::errors::Result<String> {
            let uri = env.new_string(uri)?;
            let result = env.call_method(
                state.activity.as_ref(),
                jni_str!("readLocalRomDirectoryFromRust"),
                jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
                &[JValue::Object(&uri)],
            )?;
            let json = env.cast_local::<JString>(result.l()?)?;
            json.try_to_string(env)
        })
        .map_err(|error| format!("Android content URI could not be read: {error}"))
}

fn open_file_picker_requests(
    mut activations: MessageReader<app::platform::UiFilePickerActivated>,
    mut requests: Local<AndroidFilePickerRequests>,
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

        let kind = if directory {
            PICKER_KIND_DIRECTORY
        } else if audio_file {
            PICKER_KIND_AUDIO
        } else {
            PICKER_KIND_ROM
        };
        let request_id = requests.next_request_id;
        requests.next_request_id = requests.next_request_id.wrapping_add(1).max(1);

        if open_android_file_picker(kind, request_id) {
            requests.pending.insert(request_id, activation.picker);
        } else {
            eprintln!("Android file picker request could not be opened.");
        }
    }

    for result in drain_android_file_picker_results() {
        let Some(picker) = requests.pending.remove(&result.request_id) else {
            continue;
        };
        results.write(app::platform::UiFilePickerResult {
            picker,
            value: result.value,
        });
    }
}

fn apply_android_text_input_changes(
    mut inputs: Query<
        (Entity, &mut app::platform::UiTextInput),
        (
            With<app::platform::FocusedUiElement>,
            With<app::platform::EditableUiElement>,
            Without<app::platform::DisabledUiElement>,
        ),
    >,
    mut state: ResMut<AndroidTextInputBridgeState>,
) {
    let Some(change) = drain_android_text_input_changes().pop() else {
        return;
    };
    let Ok((entity, mut input)) = inputs.single_mut() else {
        return;
    };

    let (value, cursor) = sanitize_android_text_change(&change.value, change.cursor_utf16);
    input.value = value.clone();
    input.cursor = cursor;
    state.applied_from_android = Some(AndroidTextInputStateSnapshot {
        entity: Some(entity),
        value,
        cursor,
    });
}

fn sync_android_text_input_keyboard(
    inputs: Query<
        (Entity, &app::platform::UiTextInput),
        (
            With<app::platform::FocusedUiElement>,
            With<app::platform::EditableUiElement>,
            Without<app::platform::DisabledUiElement>,
        ),
    >,
    mut state: ResMut<AndroidTextInputBridgeState>,
) {
    let snapshot = inputs
        .single()
        .ok()
        .map(|(entity, input)| AndroidTextInputStateSnapshot {
            entity: Some(entity),
            value: input.value.clone(),
            cursor: clamp_cursor_to_char_boundary(&input.value, input.cursor),
        })
        .unwrap_or_default();

    if snapshot == state.current {
        return;
    }

    if snapshot.entity.is_none() {
        if state.current.entity.is_some() {
            hide_android_text_input_keyboard();
        }
        state.current = snapshot;
        state.applied_from_android = None;
        return;
    }

    if state.current.entity != snapshot.entity {
        show_android_text_input_keyboard(&snapshot.value, snapshot.cursor);
        state.current = snapshot;
        state.applied_from_android = None;
        return;
    }

    if state.applied_from_android.as_ref() == Some(&snapshot) {
        state.current = snapshot;
        state.applied_from_android = None;
        return;
    }

    sync_android_text_input_value(&snapshot.value, snapshot.cursor);
    state.current = snapshot;
    state.applied_from_android = None;
}

fn open_android_file_picker(kind: u32, request_id: u64) -> bool {
    let Ok(state) = ANDROID_ACTIVITY.lock() else {
        eprintln!("failed to lock Android activity state");
        return false;
    };
    let Some(state) = state.as_ref() else {
        eprintln!("Android activity is not registered yet.");
        return false;
    };
    let result: jni::errors::Result<bool> = state.vm.attach_current_thread(|env| {
        let result = env.call_method(
            state.activity.as_ref(),
            jni_str!("openFilePickerFromRust"),
            jni_sig!("(JI)Z"),
            &[JValue::Long(request_id as jlong), JValue::Int(kind as jint)],
        )?;
        Ok(result.z().unwrap_or(false))
    });
    match result {
        Ok(opened) => opened,
        Err(error) => {
            eprintln!("failed to call Android file picker: {error}");
            false
        }
    }
}

fn show_android_text_input_keyboard(value: &str, cursor: usize) {
    call_android_text_input_method(jni_str!("showSoftwareKeyboard"), value, cursor);
}

fn sync_android_text_input_value(value: &str, cursor: usize) {
    call_android_text_input_method(jni_str!("syncSoftwareKeyboardText"), value, cursor);
}

fn call_android_text_input_method(
    method: &'static jni::strings::JNIStr,
    value: &str,
    cursor: usize,
) {
    let Ok(state) = ANDROID_ACTIVITY.lock() else {
        eprintln!("failed to lock Android activity state");
        return;
    };
    let Some(state) = state.as_ref() else {
        eprintln!("Android activity is not registered yet.");
        return;
    };

    let result: jni::errors::Result<()> = state.vm.attach_current_thread(|env| {
        let cursor_utf16 = utf16_cursor_for_byte_cursor(value, cursor) as jint;
        let value = env.new_string(value)?;
        env.call_method(
            state.activity.as_ref(),
            method,
            jni_sig!("(Ljava/lang/String;I)V"),
            &[JValue::Object(&value), JValue::Int(cursor_utf16)],
        )?;
        Ok(())
    });

    if let Err(error) = result {
        eprintln!("failed to sync Android text input: {error}");
    }
}

fn hide_android_text_input_keyboard() {
    let Ok(state) = ANDROID_ACTIVITY.lock() else {
        eprintln!("failed to lock Android activity state");
        return;
    };
    let Some(state) = state.as_ref() else {
        return;
    };

    let result: jni::errors::Result<()> = state.vm.attach_current_thread(|env| {
        env.call_method(
            state.activity.as_ref(),
            jni_str!("hideSoftwareKeyboard"),
            jni_sig!("()V"),
            &[],
        )?;
        Ok(())
    });

    if let Err(error) = result {
        eprintln!("failed to hide Android text input: {error}");
    }
}

fn push_android_file_picker_result(request_id: u64, value: String) {
    let Ok(mut results) = ANDROID_FILE_PICKER_RESULTS.lock() else {
        eprintln!("failed to lock Android file picker result queue");
        return;
    };
    results.push_back(AndroidFilePickerResult { request_id, value });
}

fn drain_android_file_picker_results() -> Vec<AndroidFilePickerResult> {
    let Ok(mut results) = ANDROID_FILE_PICKER_RESULTS.lock() else {
        eprintln!("failed to lock Android file picker result queue");
        return Vec::new();
    };
    results.drain(..).collect()
}

fn push_android_text_input_change(value: String, cursor_utf16: usize) {
    let Ok(mut changes) = ANDROID_TEXT_INPUT_CHANGES.lock() else {
        eprintln!("failed to lock Android text input change queue");
        return;
    };
    changes.push_back(AndroidTextInputChange {
        value,
        cursor_utf16,
    });
}

fn drain_android_text_input_changes() -> Vec<AndroidTextInputChange> {
    let Ok(mut changes) = ANDROID_TEXT_INPUT_CHANGES.lock() else {
        eprintln!("failed to lock Android text input change queue");
        return Vec::new();
    };
    changes.drain(..).collect()
}

fn sanitize_android_text_change(value: &str, cursor_utf16: usize) -> (String, usize) {
    let cursor_byte = byte_cursor_for_utf16_cursor(value, cursor_utf16);
    let before_cursor = sanitize_single_line_text(&value[..cursor_byte]);
    let after_cursor = sanitize_single_line_text(&value[cursor_byte..]);
    let cursor = before_cursor.len();
    (format!("{before_cursor}{after_cursor}"), cursor)
}

fn sanitize_single_line_text(value: &str) -> String {
    value
        .chars()
        .filter_map(|character| match character {
            '\r' | '\n' => None,
            '\t' => Some(' '),
            character if character.is_control() => None,
            character => Some(character),
        })
        .collect()
}

fn clamp_cursor_to_char_boundary(value: &str, cursor: usize) -> usize {
    let cursor = cursor.min(value.len());
    if value.is_char_boundary(cursor) {
        cursor
    } else {
        value
            .char_indices()
            .map(|(index, _)| index)
            .take_while(|index| *index < cursor)
            .last()
            .unwrap_or(0)
    }
}

fn byte_cursor_for_utf16_cursor(value: &str, cursor_utf16: usize) -> usize {
    let mut units = 0;
    for (index, character) in value.char_indices() {
        let next_units = units + character.len_utf16();
        if next_units > cursor_utf16 {
            return index;
        }
        units = next_units;
    }
    value.len()
}

fn utf16_cursor_for_byte_cursor(value: &str, cursor: usize) -> usize {
    let cursor = clamp_cursor_to_char_boundary(value, cursor);
    value[..cursor].encode_utf16().count()
}
