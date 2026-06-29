package com.shininggrimace.shiningemulator.managers

import android.hardware.input.InputManager
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import com.shininggrimace.shiningemulator.interfaces.NativeControllerReporter
import com.shininggrimace.shiningemulator.utils.ControllerHatState

object ControllerManager {

    private const val GAMEPAD_BUTTON_SOUTH = 0
    private const val GAMEPAD_BUTTON_EAST = 1
    private const val GAMEPAD_BUTTON_NORTH = 2
    private const val GAMEPAD_BUTTON_WEST = 3
    private const val GAMEPAD_BUTTON_C = 4
    private const val GAMEPAD_BUTTON_Z = 5
    private const val GAMEPAD_BUTTON_LEFT_TRIGGER = 6
    private const val GAMEPAD_BUTTON_LEFT_TRIGGER_2 = 7
    private const val GAMEPAD_BUTTON_RIGHT_TRIGGER = 8
    private const val GAMEPAD_BUTTON_RIGHT_TRIGGER_2 = 9
    private const val GAMEPAD_BUTTON_SELECT = 10
    private const val GAMEPAD_BUTTON_START = 11
    private const val GAMEPAD_BUTTON_MODE = 12
    private const val GAMEPAD_BUTTON_LEFT_THUMB = 13
    private const val GAMEPAD_BUTTON_RIGHT_THUMB = 14
    private const val GAMEPAD_BUTTON_DPAD_UP = 15
    private const val GAMEPAD_BUTTON_DPAD_DOWN = 16
    private const val GAMEPAD_BUTTON_DPAD_LEFT = 17
    private const val GAMEPAD_BUTTON_DPAD_RIGHT = 18

    private const val GAMEPAD_AXIS_LEFT_STICK_X = 0
    private const val GAMEPAD_AXIS_LEFT_STICK_Y = 1
    private const val GAMEPAD_AXIS_RIGHT_STICK_X = 2
    private const val GAMEPAD_AXIS_RIGHT_STICK_Y = 3

    private var listener: InputManager.InputDeviceListener? = null
    private val connectedControllerIds = mutableSetOf<Int>()
    private val controllerHatStates = mutableMapOf<Int, ControllerHatState>()

    fun deviceListener(callbacks: NativeControllerReporter): InputManager.InputDeviceListener {
        listener?.let {
            return it
        }
        return newDeviceListener(callbacks).apply {
            listener = this
        }
    }

    fun NativeControllerReporter.ensureAllControllersConnected() {
        for (deviceId in InputDevice.getDeviceIds()) {
            ensureControllerConnected(InputDevice.getDevice(deviceId))
        }
    }

    private fun newDeviceListener(
        callbacks: NativeControllerReporter
    ) = object: InputManager.InputDeviceListener {

        override fun onInputDeviceAdded(deviceId: Int) {
            callbacks.ensureControllerConnected(InputDevice.getDevice(deviceId))
        }

        override fun onInputDeviceRemoved(deviceId: Int) {
            if (connectedControllerIds.remove(deviceId)) {
                controllerHatStates.remove(deviceId)
                callbacks.onControllerDisconnected(deviceId)
            }
        }

        override fun onInputDeviceChanged(deviceId: Int) {
            val device = InputDevice.getDevice(deviceId)
            if (!isSupportedControllerDevice(device)) {
                onInputDeviceRemoved(deviceId)
                return
            }

            if (connectedControllerIds.remove(deviceId)) {
                callbacks.onControllerDisconnected(deviceId)
            }
            callbacks.ensureControllerConnected(device)
        }
    }

    private fun NativeControllerReporter.ensureControllerConnected(device: InputDevice?): Boolean {
        val deviceId = device?.id ?: return false
        if (!isSupportedControllerDevice(device)) {
            return false
        }
        if (!connectedControllerIds.add(deviceId)) {
            return true
        }

        onControllerConnected(
            deviceId,
            device.name,
            device.vendorId,
            device.productId
        )
        return true
    }

    private fun isSupportedControllerDevice(device: InputDevice?): Boolean {
        return device != null && !device.isVirtual && isControllerSource(device.sources)
    }

    private fun isControllerSource(source: Int): Boolean {
        return (source and InputDevice.SOURCE_GAMEPAD) == InputDevice.SOURCE_GAMEPAD ||
                (source and InputDevice.SOURCE_JOYSTICK) == InputDevice.SOURCE_JOYSTICK ||
                (source and InputDevice.SOURCE_DPAD) == InputDevice.SOURCE_DPAD
    }

    private fun gamepadButtonForKeyCode(keyCode: Int): Int? {
        return when (keyCode) {
            KeyEvent.KEYCODE_BUTTON_A -> GAMEPAD_BUTTON_SOUTH
            KeyEvent.KEYCODE_BUTTON_B -> GAMEPAD_BUTTON_EAST
            KeyEvent.KEYCODE_BUTTON_Y -> GAMEPAD_BUTTON_NORTH
            KeyEvent.KEYCODE_BUTTON_X -> GAMEPAD_BUTTON_WEST
            KeyEvent.KEYCODE_BUTTON_C -> GAMEPAD_BUTTON_C
            KeyEvent.KEYCODE_BUTTON_Z -> GAMEPAD_BUTTON_Z
            KeyEvent.KEYCODE_BUTTON_L1 -> GAMEPAD_BUTTON_LEFT_TRIGGER
            KeyEvent.KEYCODE_BUTTON_L2 -> GAMEPAD_BUTTON_LEFT_TRIGGER_2
            KeyEvent.KEYCODE_BUTTON_R1 -> GAMEPAD_BUTTON_RIGHT_TRIGGER
            KeyEvent.KEYCODE_BUTTON_R2 -> GAMEPAD_BUTTON_RIGHT_TRIGGER_2
            KeyEvent.KEYCODE_BUTTON_SELECT, KeyEvent.KEYCODE_BACK -> GAMEPAD_BUTTON_SELECT
            KeyEvent.KEYCODE_BUTTON_START -> GAMEPAD_BUTTON_START
            KeyEvent.KEYCODE_BUTTON_MODE -> GAMEPAD_BUTTON_MODE
            KeyEvent.KEYCODE_BUTTON_THUMBL -> GAMEPAD_BUTTON_LEFT_THUMB
            KeyEvent.KEYCODE_BUTTON_THUMBR -> GAMEPAD_BUTTON_RIGHT_THUMB
            KeyEvent.KEYCODE_DPAD_UP -> GAMEPAD_BUTTON_DPAD_UP
            KeyEvent.KEYCODE_DPAD_DOWN -> GAMEPAD_BUTTON_DPAD_DOWN
            KeyEvent.KEYCODE_DPAD_LEFT -> GAMEPAD_BUTTON_DPAD_LEFT
            KeyEvent.KEYCODE_DPAD_RIGHT -> GAMEPAD_BUTTON_DPAD_RIGHT
            else -> null
        }
    }

    fun NativeControllerReporter.handleControllerKeyEvent(event: KeyEvent): Boolean {
        if (event.action != KeyEvent.ACTION_DOWN && event.action != KeyEvent.ACTION_UP) {
            return false
        }
        if (!isControllerSource(event.source)) {
            return false
        }
        if (event.repeatCount > 0) {
            return true
        }

        val button = gamepadButtonForKeyCode(event.keyCode) ?: return false
        if (!ensureControllerConnected(event.device)) {
            return true
        }
        onControllerButtonChanged(
            event.deviceId,
            button,
            if (event.action == KeyEvent.ACTION_DOWN) 1.0f else 0.0f
        )
        return true
    }

    fun NativeControllerReporter.handleControllerMotionEvent(event: MotionEvent): Boolean {
        if (event.action != MotionEvent.ACTION_MOVE || !isControllerSource(event.source)) {
            return false
        }

        if (!ensureControllerConnected(event.device)) {
            return true
        }
        onControllerAxisChanged(
            event.deviceId,
            GAMEPAD_AXIS_LEFT_STICK_X,
            event.getAxisValue(MotionEvent.AXIS_X)
        )
        onControllerAxisChanged(
            event.deviceId,
            GAMEPAD_AXIS_LEFT_STICK_Y,
            -event.getAxisValue(MotionEvent.AXIS_Y)
        )
        onControllerAxisChanged(
            event.deviceId,
            GAMEPAD_AXIS_RIGHT_STICK_X,
            event.getAxisValue(MotionEvent.AXIS_Z)
        )
        onControllerAxisChanged(
            event.deviceId,
            GAMEPAD_AXIS_RIGHT_STICK_Y,
            -event.getAxisValue(MotionEvent.AXIS_RZ)
        )
        onControllerButtonChanged(
            event.deviceId,
            GAMEPAD_BUTTON_LEFT_TRIGGER_2,
            triggerValue(event, MotionEvent.AXIS_LTRIGGER, MotionEvent.AXIS_BRAKE)
        )
        onControllerButtonChanged(
            event.deviceId,
            GAMEPAD_BUTTON_RIGHT_TRIGGER_2,
            triggerValue(event, MotionEvent.AXIS_RTRIGGER, MotionEvent.AXIS_GAS)
        )
        updateControllerHatButtons(
            event.deviceId,
            event.getAxisValue(MotionEvent.AXIS_HAT_X),
            event.getAxisValue(MotionEvent.AXIS_HAT_Y)
        )
        return true
    }

    private fun triggerValue(event: MotionEvent, primaryAxis: Int, fallbackAxis: Int): Float {
        val primaryValue = event.getAxisValue(primaryAxis)
        if (primaryValue != 0.0f) {
            return primaryValue.coerceIn(0.0f, 1.0f)
        }
        return event.getAxisValue(fallbackAxis).coerceIn(0.0f, 1.0f)
    }

    private fun NativeControllerReporter.updateControllerHatButtons(deviceId: Int, hatX: Float, hatY: Float) {
        val oldState = controllerHatStates[deviceId] ?: ControllerHatState()
        val newState = ControllerHatState(
            left = hatX < -0.5f,
            right = hatX > 0.5f,
            up = hatY < -0.5f,
            down = hatY > 0.5f
        )
        controllerHatStates[deviceId] = newState

        emitHatButtonChange(deviceId, GAMEPAD_BUTTON_DPAD_LEFT, oldState.left, newState.left)
        emitHatButtonChange(deviceId, GAMEPAD_BUTTON_DPAD_RIGHT, oldState.right, newState.right)
        emitHatButtonChange(deviceId, GAMEPAD_BUTTON_DPAD_UP, oldState.up, newState.up)
        emitHatButtonChange(deviceId, GAMEPAD_BUTTON_DPAD_DOWN, oldState.down, newState.down)
    }

    private fun NativeControllerReporter.emitHatButtonChange(deviceId: Int, button: Int, oldValue: Boolean, newValue: Boolean) {
        if (oldValue != newValue) {
            onControllerButtonChanged(deviceId, button, if (newValue) 1.0f else 0.0f)
        }
    }
}