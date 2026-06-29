package com.shininggrimace.shiningemulator.interfaces

interface NativeControllerReporter {
    fun onControllerConnected(
        deviceId: Int,
        name: String?,
        vendorId: Int,
        productId: Int
    )
    fun onControllerDisconnected(deviceId: Int)
    fun onControllerButtonChanged(deviceId: Int, button: Int, value: Float)
    fun onControllerAxisChanged(deviceId: Int, axis: Int, value: Float)
}