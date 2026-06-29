use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, Mutex};

use base64::Engine;
use bevy::input::InputSystems;
use bevy::input::gamepad::{
    GamepadAxis, GamepadButton, GamepadConnection, GamepadConnectionEvent,
    RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::{prelude::*, winit::WinitSettings};
use jni::errors::ThrowRuntimeExAndDefault;
use jni::objects::{Global, JObject, JString, JValue};
use jni::sys::{jfloat, jint, jlong};
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

#[derive(Clone, Debug)]
enum AndroidControllerEvent {
    Connected {
        device_id: i32,
        name: String,
        vendor_id: Option<u16>,
        product_id: Option<u16>,
    },
    Disconnected {
        device_id: i32,
    },
    ButtonChanged {
        device_id: i32,
        button: AndroidGamepadButton,
        value: f32,
    },
    AxisChanged {
        device_id: i32,
        axis: AndroidGamepadAxis,
        value: f32,
    },
}

#[derive(Clone, Copy, Debug)]
enum AndroidGamepadButton {
    South,
    East,
    North,
    West,
    C,
    Z,
    LeftTrigger,
    LeftTrigger2,
    RightTrigger,
    RightTrigger2,
    Select,
    Start,
    Mode,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

#[derive(Clone, Copy, Debug)]
enum AndroidGamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
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

#[derive(Default, Resource)]
struct AndroidControllerBridgeState {
    devices: HashMap<i32, Entity>,
}

static ANDROID_ACTIVITY: LazyLock<Mutex<Option<AndroidActivity>>> =
    LazyLock::new(|| Mutex::new(None));

static ANDROID_FILE_PICKER_RESULTS: LazyLock<Mutex<VecDeque<AndroidFilePickerResult>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));

static ANDROID_TEXT_INPUT_CHANGES: LazyLock<Mutex<VecDeque<AndroidTextInputChange>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));

static ANDROID_CONTROLLER_EVENTS: LazyLock<Mutex<VecDeque<AndroidControllerEvent>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));

impl Plugin for MobilePlugin {
    // Sets 60 fps; note this can be changed at runtime if needed
    fn build(&self, app: &mut App) {
        app::platform::set_android_local_directory_reader(read_android_local_directory_roms);
        app.insert_resource(WinitSettings::mobile())
            .init_resource::<AndroidTextInputBridgeState>()
            .init_resource::<AndroidControllerBridgeState>()
            .add_systems(
                PreUpdate,
                forward_android_controller_events.before(InputSystems),
            )
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

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_shiningemulator_MainActivity_nativeOnControllerConnected<
    'local,
>(
    mut env: EnvUnowned<'local>,
    _activity: JObject<'local>,
    device_id: jint,
    name: JString<'local>,
    vendor_id: jint,
    product_id: jint,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        let name = if name.is_null() {
            format!("Android Controller {device_id}")
        } else {
            name.try_to_string(env)?
        };
        push_android_controller_event(AndroidControllerEvent::Connected {
            device_id,
            name,
            vendor_id: android_usb_id(vendor_id),
            product_id: android_usb_id(product_id),
        });
        Ok(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_shiningemulator_MainActivity_nativeOnControllerDisconnected(
    _env: EnvUnowned<'_>,
    _activity: JObject<'_>,
    device_id: jint,
) {
    push_android_controller_event(AndroidControllerEvent::Disconnected { device_id });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_shiningemulator_MainActivity_nativeOnControllerButtonChanged(
    _env: EnvUnowned<'_>,
    _activity: JObject<'_>,
    device_id: jint,
    button: jint,
    value: jfloat,
) {
    let Some(button) = AndroidGamepadButton::from_jint(button) else {
        return;
    };
    push_android_controller_event(AndroidControllerEvent::ButtonChanged {
        device_id,
        button,
        value,
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_shiningemulator_MainActivity_nativeOnControllerAxisChanged(
    _env: EnvUnowned<'_>,
    _activity: JObject<'_>,
    device_id: jint,
    axis: jint,
    value: jfloat,
) {
    let Some(axis) = AndroidGamepadAxis::from_jint(axis) else {
        return;
    };
    push_android_controller_event(AndroidControllerEvent::AxisChanged {
        device_id,
        axis,
        value,
    });
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

fn forward_android_controller_events(
    mut commands: Commands,
    mut state: ResMut<AndroidControllerBridgeState>,
    mut raw_events: MessageWriter<RawGamepadEvent>,
    mut connection_events: MessageWriter<GamepadConnectionEvent>,
    mut raw_button_events: MessageWriter<RawGamepadButtonChangedEvent>,
    mut raw_axis_events: MessageWriter<RawGamepadAxisChangedEvent>,
) {
    for event in drain_android_controller_events() {
        match event {
            AndroidControllerEvent::Connected {
                device_id,
                name,
                vendor_id,
                product_id,
            } => {
                if state.devices.contains_key(&device_id) {
                    continue;
                }

                let entity = commands.spawn_empty().id();
                state.devices.insert(device_id, entity);
                let event = GamepadConnectionEvent::new(
                    entity,
                    GamepadConnection::Connected {
                        name,
                        vendor_id,
                        product_id,
                    },
                );
                raw_events.write(event.clone().into());
                connection_events.write(event);
            }
            AndroidControllerEvent::Disconnected { device_id } => {
                let Some(entity) = state.devices.remove(&device_id) else {
                    continue;
                };
                let event = GamepadConnectionEvent::new(entity, GamepadConnection::Disconnected);
                raw_events.write(event.clone().into());
                connection_events.write(event);
            }
            AndroidControllerEvent::ButtonChanged {
                device_id,
                button,
                value,
            } => {
                let Some(entity) = android_controller_entity(
                    &mut commands,
                    &mut state,
                    device_id,
                    &mut raw_events,
                    &mut connection_events,
                ) else {
                    continue;
                };
                let raw_button = RawGamepadButtonChangedEvent::new(
                    entity,
                    button.into(),
                    clamp_button_value(value),
                );
                raw_events.write(raw_button.into());
                raw_button_events.write(raw_button);
            }
            AndroidControllerEvent::AxisChanged {
                device_id,
                axis,
                value,
            } => {
                let Some(entity) = android_controller_entity(
                    &mut commands,
                    &mut state,
                    device_id,
                    &mut raw_events,
                    &mut connection_events,
                ) else {
                    continue;
                };
                let raw_axis =
                    RawGamepadAxisChangedEvent::new(entity, axis.into(), clamp_axis_value(value));
                raw_events.write(raw_axis.into());
                raw_axis_events.write(raw_axis);
            }
        }
    }
}

fn android_controller_entity(
    commands: &mut Commands,
    state: &mut AndroidControllerBridgeState,
    device_id: i32,
    raw_events: &mut MessageWriter<RawGamepadEvent>,
    connection_events: &mut MessageWriter<GamepadConnectionEvent>,
) -> Option<Entity> {
    if let Some(entity) = state.devices.get(&device_id) {
        return Some(*entity);
    }

    let entity = commands.spawn_empty().id();
    state.devices.insert(device_id, entity);
    let event = GamepadConnectionEvent::new(
        entity,
        GamepadConnection::Connected {
            name: format!("Android Controller {device_id}"),
            vendor_id: None,
            product_id: None,
        },
    );
    raw_events.write(event.clone().into());
    connection_events.write(event);
    Some(entity)
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

fn push_android_controller_event(event: AndroidControllerEvent) {
    let Ok(mut events) = ANDROID_CONTROLLER_EVENTS.lock() else {
        eprintln!("failed to lock Android controller event queue");
        return;
    };
    events.push_back(event);
}

fn drain_android_controller_events() -> Vec<AndroidControllerEvent> {
    let Ok(mut events) = ANDROID_CONTROLLER_EVENTS.lock() else {
        eprintln!("failed to lock Android controller event queue");
        return Vec::new();
    };
    events.drain(..).collect()
}

fn android_usb_id(value: i32) -> Option<u16> {
    u16::try_from(value).ok().filter(|value| *value != 0)
}

fn clamp_button_value(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn clamp_axis_value(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-1.0, 1.0)
    } else {
        0.0
    }
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

impl AndroidGamepadButton {
    fn from_jint(value: jint) -> Option<Self> {
        match value {
            0 => Some(Self::South),
            1 => Some(Self::East),
            2 => Some(Self::North),
            3 => Some(Self::West),
            4 => Some(Self::C),
            5 => Some(Self::Z),
            6 => Some(Self::LeftTrigger),
            7 => Some(Self::LeftTrigger2),
            8 => Some(Self::RightTrigger),
            9 => Some(Self::RightTrigger2),
            10 => Some(Self::Select),
            11 => Some(Self::Start),
            12 => Some(Self::Mode),
            13 => Some(Self::LeftThumb),
            14 => Some(Self::RightThumb),
            15 => Some(Self::DPadUp),
            16 => Some(Self::DPadDown),
            17 => Some(Self::DPadLeft),
            18 => Some(Self::DPadRight),
            _ => None,
        }
    }
}

impl From<AndroidGamepadButton> for GamepadButton {
    fn from(value: AndroidGamepadButton) -> Self {
        match value {
            AndroidGamepadButton::South => GamepadButton::South,
            AndroidGamepadButton::East => GamepadButton::East,
            AndroidGamepadButton::North => GamepadButton::North,
            AndroidGamepadButton::West => GamepadButton::West,
            AndroidGamepadButton::C => GamepadButton::C,
            AndroidGamepadButton::Z => GamepadButton::Z,
            AndroidGamepadButton::LeftTrigger => GamepadButton::LeftTrigger,
            AndroidGamepadButton::LeftTrigger2 => GamepadButton::LeftTrigger2,
            AndroidGamepadButton::RightTrigger => GamepadButton::RightTrigger,
            AndroidGamepadButton::RightTrigger2 => GamepadButton::RightTrigger2,
            AndroidGamepadButton::Select => GamepadButton::Select,
            AndroidGamepadButton::Start => GamepadButton::Start,
            AndroidGamepadButton::Mode => GamepadButton::Mode,
            AndroidGamepadButton::LeftThumb => GamepadButton::LeftThumb,
            AndroidGamepadButton::RightThumb => GamepadButton::RightThumb,
            AndroidGamepadButton::DPadUp => GamepadButton::DPadUp,
            AndroidGamepadButton::DPadDown => GamepadButton::DPadDown,
            AndroidGamepadButton::DPadLeft => GamepadButton::DPadLeft,
            AndroidGamepadButton::DPadRight => GamepadButton::DPadRight,
        }
    }
}

impl AndroidGamepadAxis {
    fn from_jint(value: jint) -> Option<Self> {
        match value {
            0 => Some(Self::LeftStickX),
            1 => Some(Self::LeftStickY),
            2 => Some(Self::RightStickX),
            3 => Some(Self::RightStickY),
            _ => None,
        }
    }
}

impl From<AndroidGamepadAxis> for GamepadAxis {
    fn from(value: AndroidGamepadAxis) -> Self {
        match value {
            AndroidGamepadAxis::LeftStickX => GamepadAxis::LeftStickX,
            AndroidGamepadAxis::LeftStickY => GamepadAxis::LeftStickY,
            AndroidGamepadAxis::RightStickX => GamepadAxis::RightStickX,
            AndroidGamepadAxis::RightStickY => GamepadAxis::RightStickY,
        }
    }
}
