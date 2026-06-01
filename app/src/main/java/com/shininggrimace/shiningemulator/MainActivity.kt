package com.shininggrimace.shiningemulator

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.net.Uri
import android.os.Bundle
import android.provider.DocumentsContract
import android.text.Editable
import android.text.InputType
import android.text.TextWatcher
import android.util.Base64
import android.view.View
import android.view.ViewGroup
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
import androidx.activity.result.contract.ActivityResultContracts
import com.google.androidgamesdk.GameActivity
import org.json.JSONArray
import org.json.JSONObject
import java.util.Locale

class MainActivity : GameActivity() {
    companion object {
        init {
            System.loadLibrary("shiningemulator")
        }

        private const val PICKER_KIND_ROM = 0
        private const val PICKER_KIND_DIRECTORY = 1
        private const val PICKER_KIND_AUDIO = 2
    }

    private val pendingFilePickerRequests = mutableListOf<Long>()
    private var softwareKeyboardInput: RustTextInputEditText? = null
    private var suppressTextInputCallbacks = false

    private val filePickerLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        val requestId = if (pendingFilePickerRequests.isNotEmpty()) {
            pendingFilePickerRequests.removeAt(0)
        } else {
            return@registerForActivityResult
        }

        if (result.resultCode != RESULT_OK) {
            nativeOnFilePickerResult(requestId, null)
            return@registerForActivityResult
        }

        val data = result.data
        val uri = data?.data
        if (uri != null) {
            takePersistablePermission(uri, data.flags)
        }
        nativeOnFilePickerResult(requestId, uri?.toString())
    }

    private external fun nativeSetActivity(activity: MainActivity)
    private external fun nativeOnFilePickerResult(requestId: Long, uri: String?)
    private external fun nativeOnTextInputChanged(value: String?, cursor: Int)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        nativeSetActivity(this)
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) {
            hideSystemUi()
        }
    }

    override fun onPause() {
        hideSoftwareKeyboard()
        super.onPause()
    }

    @Suppress("unused")
    fun openFilePickerFromRust(requestId: Long, kind: Int): Boolean {
        val intent = when (kind) {
            PICKER_KIND_DIRECTORY -> Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).apply {
                addFlags(persistablePickerFlags())
            }
            PICKER_KIND_AUDIO -> Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "audio/*"
                putExtra(Intent.EXTRA_MIME_TYPES, arrayOf("audio/wav", "audio/x-wav"))
                addFlags(persistablePickerFlags())
            }
            PICKER_KIND_ROM -> Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE)
                type = "application/octet-stream"
                addFlags(persistablePickerFlags())
            }
            else -> return false
        }

        runOnUiThread {
            try {
                pendingFilePickerRequests.add(requestId)
                filePickerLauncher.launch(intent)
            } catch (_: ActivityNotFoundException) {
                pendingFilePickerRequests.remove(requestId)
                nativeOnFilePickerResult(requestId, null)
            }
        }
        return true
    }

    @Suppress("unused")
    fun readLocalRomDirectoryFromRust(uriText: String): String {
        val treeUri = Uri.parse(uriText)
        val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(
            treeUri,
            DocumentsContract.getTreeDocumentId(treeUri)
        )
        val projection = arrayOf(
            DocumentsContract.Document.COLUMN_DOCUMENT_ID,
            DocumentsContract.Document.COLUMN_DISPLAY_NAME,
            DocumentsContract.Document.COLUMN_MIME_TYPE
        )
        val roms = JSONArray()
        val cursor = contentResolver.query(childrenUri, projection, null, null, null)
            ?: throw IllegalArgumentException("Directory could not be opened.")
        cursor.use {
            val documentIdColumn = it.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DOCUMENT_ID)
            val displayNameColumn = it.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
            val mimeTypeColumn = it.getColumnIndexOrThrow(DocumentsContract.Document.COLUMN_MIME_TYPE)

            while (it.moveToNext()) {
                val mimeType = it.getString(mimeTypeColumn)
                if (mimeType == DocumentsContract.Document.MIME_TYPE_DIR) {
                    continue
                }

                val fileName = it.getString(displayNameColumn) ?: continue
                if (!isRomFileName(fileName)) {
                    continue
                }

                val documentUri = DocumentsContract.buildDocumentUriUsingTree(
                    treeUri,
                    it.getString(documentIdColumn)
                )
                val bytes = contentResolver.openInputStream(documentUri)?.use { stream ->
                    stream.readBytes()
                } ?: continue

                roms.put(
                    JSONObject()
                        .put("fileName", fileName)
                        .put("base64", Base64.encodeToString(bytes, Base64.NO_WRAP))
                )
            }
        }
        return roms.toString()
    }

    @Suppress("unused")
    fun showSoftwareKeyboard(value: String, cursor: Int) {
        runOnUiThread {
            val input = ensureSoftwareKeyboardInput()
            setSoftwareKeyboardText(input, value, cursor)
            input.requestFocus()
            input.post {
                input.requestFocus()
                inputMethodManager().showSoftInput(input, InputMethodManager.SHOW_IMPLICIT)
            }
        }
    }

    @Suppress("unused")
    fun syncSoftwareKeyboardText(value: String, cursor: Int) {
        runOnUiThread {
            setSoftwareKeyboardText(ensureSoftwareKeyboardInput(), value, cursor)
        }
    }

    @Suppress("unused")
    fun hideSoftwareKeyboard() {
        runOnUiThread {
            val input = softwareKeyboardInput ?: return@runOnUiThread
            inputMethodManager().hideSoftInputFromWindow(input.windowToken, 0)
            input.clearFocus()
            hideSystemUi()
        }
    }

    private fun hideSystemUi() {
        val decorView = window.decorView
        decorView.systemUiVisibility = (View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                or View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                or View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                or View.SYSTEM_UI_FLAG_FULLSCREEN)
    }

    private fun ensureSoftwareKeyboardInput(): RustTextInputEditText {
        val existing = softwareKeyboardInput
        if (existing != null) {
            return existing
        }

        val input = RustTextInputEditText(this).apply {
            alpha = 0.01f
            setBackgroundColor(Color.TRANSPARENT)
            setTextColor(Color.TRANSPARENT)
            setHintTextColor(Color.TRANSPARENT)
            isCursorVisible = false
            isSingleLine = true
            inputType = (InputType.TYPE_CLASS_TEXT
                    or InputType.TYPE_TEXT_VARIATION_URI
                    or InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS)
            imeOptions = EditorInfo.IME_ACTION_DONE or EditorInfo.IME_FLAG_NO_FULLSCREEN
            addTextChangedListener(object : TextWatcher {
                override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
                override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) = Unit

                override fun afterTextChanged(s: Editable?) {
                    notifyTextInputChanged()
                }
            })
            setOnEditorActionListener { _, actionId, _ ->
                if (actionId == EditorInfo.IME_ACTION_DONE) {
                    hideSoftwareKeyboard()
                    true
                } else {
                    false
                }
            }
        }
        softwareKeyboardInput = input
        addContentView(input, ViewGroup.LayoutParams(1, 1))
        return input
    }

    private fun setSoftwareKeyboardText(input: EditText, value: String, cursor: Int) {
        suppressTextInputCallbacks = true
        if (input.text?.toString() != value) {
            input.setText(value)
        }
        val clampedCursor = cursor.coerceIn(0, value.length)
        if (input.selectionStart != clampedCursor || input.selectionEnd != clampedCursor) {
            input.setSelection(clampedCursor)
        }
        suppressTextInputCallbacks = false
    }

    private fun notifyTextInputChanged() {
        if (suppressTextInputCallbacks) {
            return
        }
        val input = softwareKeyboardInput ?: return
        nativeOnTextInputChanged(input.text?.toString(), input.selectionStart.coerceAtLeast(0))
    }

    private fun inputMethodManager(): InputMethodManager {
        return getSystemService(INPUT_METHOD_SERVICE) as InputMethodManager
    }

    private inner class RustTextInputEditText(context: Context) : EditText(context) {
        override fun onSelectionChanged(selStart: Int, selEnd: Int) {
            super.onSelectionChanged(selStart, selEnd)
            notifyTextInputChanged()
        }
    }

    private fun persistablePickerFlags(): Int {
        return Intent.FLAG_GRANT_READ_URI_PERMISSION or
                Intent.FLAG_GRANT_WRITE_URI_PERMISSION or
                Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION or
                Intent.FLAG_GRANT_PREFIX_URI_PERMISSION
    }

    private fun takePersistablePermission(uri: Uri, resultFlags: Int) {
        val permissionFlags = resultFlags and
                (Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
        if (permissionFlags == 0) {
            return
        }
        try {
            contentResolver.takePersistableUriPermission(uri, permissionFlags)
        } catch (_: SecurityException) {
            // Some providers grant temporary access only.
        }
    }

    private fun isRomFileName(fileName: String): Boolean {
        val lowerName = fileName.lowercase(Locale.ROOT)
        return lowerName.endsWith(".gb") || lowerName.endsWith(".gbc")
    }
}
