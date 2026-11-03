package com.estigio.proxybuilder.extra

import android.app.Activity
import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.net.wifi.WifiManager
import android.os.Build
import android.provider.OpenableColumns
import android.provider.Settings
import android.view.WindowManager
import androidx.core.content.FileProvider
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Channel
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File

@InvokeArg
class CompartirArgs {
    lateinit var ruta: String
    var tipo: String = "*/*"
}

@InvokeArg
class SiArgs {
    var si: Boolean = false
}

@InvokeArg
class RutaArgs {
    lateinit var ruta: String
}

@InvokeArg
class UriArgs {
    lateinit var uri: String
}

@InvokeArg
class EscuchaArgs {
    lateinit var handler: Channel
}

/**
 * Lo que solo hace Android. 🛑 Solo lo llama Rust (`src/android.rs` de la app);
 * la vista web no tiene permisos sobre este plugin. Los ficheros que se
 * comparten o instalan están en la caché de la app, que el `FileProvider` de la
 * plantilla (`res/xml/file_paths.xml`, `<cache-path>`) ya expone.
 */
@TauriPlugin
class AndroidExtraPlugin(private val activity: Activity) : Plugin(activity) {
    private var multicastLock: WifiManager.MulticastLock? = null
    private var canal: Channel? = null

    /** El último fichero que otra app ha abierto con esta y nadie ha recogido. */
    private var pendiente: String? = null

    private fun autoridad() = "${activity.packageName}.fileprovider"

    private fun uriDe(ruta: String): Uri =
        FileProvider.getUriForFile(activity, autoridad(), File(ruta))

    // ── Compartir ───────────────────────────────────────────────────────

    @Command
    fun compartir(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(CompartirArgs::class.java)
            val uri = uriDe(args.ruta)
            val envio = Intent(Intent.ACTION_SEND).apply {
                type = args.tipo
                putExtra(Intent.EXTRA_STREAM, uri)
                clipData = ClipData.newRawUri(File(args.ruta).name, uri)
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
            val selector = Intent.createChooser(envio, null).apply {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            activity.startActivity(selector)
            invoke.resolve()
        } catch (ex: Exception) {
            invoke.reject(ex.message ?: "no se pudo compartir")
        }
    }

    // ── La ventana ──────────────────────────────────────────────────────

    @Command
    fun pantallaEncendida(invoke: Invoke) {
        val args = invoke.parseArgs(SiArgs::class.java)
        activity.runOnUiThread {
            val ventana = activity.window
            if (args.si) {
                ventana.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            } else {
                ventana.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            }
            invoke.resolve()
        }
    }

    @Command
    fun pantallaCompleta(invoke: Invoke) {
        val args = invoke.parseArgs(SiArgs::class.java)
        activity.runOnUiThread {
            try {
                val ventana = activity.window
                val control = WindowCompat.getInsetsController(ventana, ventana.decorView)
                if (args.si) {
                    // Un gesto desde el borde las enseña un momento; «atrás» o ese gesto
                    // las devuelven del todo.
                    control.systemBarsBehavior =
                        WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
                    control.hide(WindowInsetsCompat.Type.systemBars())
                } else {
                    control.show(WindowInsetsCompat.Type.systemBars())
                }
                invoke.resolve()
            } catch (ex: Exception) {
                invoke.reject(ex.message ?: "no se pudo cambiar la pantalla completa")
            }
        }
    }

    // ── Wi-Fi ───────────────────────────────────────────────────────────

    @Command
    fun multicast(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(SiArgs::class.java)
            synchronized(this) {
                if (args.si) {
                    if (multicastLock == null) {
                        val wifi = activity.applicationContext
                            .getSystemService(Context.WIFI_SERVICE) as WifiManager
                        // Sin contador aquí: quién lo usa (buscar, recibir) lo cuenta
                        // Rust (`android::multicast`) y solo pide soltarlo con el último.
                        multicastLock = wifi.createMulticastLock("proxy-builder-multicast").apply {
                            setReferenceCounted(false)
                        }
                    }
                    if (multicastLock?.isHeld != true) multicastLock?.acquire()
                } else if (multicastLock?.isHeld == true) {
                    multicastLock?.release()
                }
            }
            invoke.resolve()
        } catch (ex: Exception) {
            invoke.reject(ex.message ?: "no se pudo cambiar el MulticastLock")
        }
    }

    override fun onDestroy() {
        synchronized(this) {
            if (multicastLock?.isHeld == true) multicastLock?.release()
        }
    }

    // ── Instalar un APK ─────────────────────────────────────────────────

    @Command
    fun instalarApk(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(RutaArgs::class.java)
            val salida = JSObject()
            if (!activity.packageManager.canRequestPackageInstalls()) {
                // Android pide el permiso en un ajuste, no en un diálogo: se abre el de
                // esta app y la persona vuelve y repite.
                val ajuste = Intent(
                    Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                    Uri.parse("package:${activity.packageName}"),
                ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                activity.startActivity(ajuste)
                salida.put("lanzado", false)
                invoke.resolve(salida)
                return
            }
            val uri = uriDe(args.ruta)
            val instalar = Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(uri, "application/vnd.android.package-archive")
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            activity.startActivity(instalar)
            salida.put("lanzado", true)
            invoke.resolve(salida)
        } catch (ex: Exception) {
            invoke.reject(ex.message ?: "no se pudo abrir el instalador")
        }
    }

    // ── Nombre de un content:// ─────────────────────────────────────────

    @Command
    fun nombreDeUri(invoke: Invoke) {
        val salida = JSObject()
        try {
            val args = invoke.parseArgs(UriArgs::class.java)
            val uri = Uri.parse(args.uri)
            activity.contentResolver.query(
                uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null,
            )?.use { c ->
                if (c.moveToFirst()) {
                    val i = c.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                    if (i >= 0) salida.put("nombre", c.getString(i))
                }
            }
        } catch (ex: Exception) {
            // Sin nombre: Rust se las apaña con lo que haya en la URI.
        }
        invoke.resolve(salida)
    }

    // ── Abrir un fichero desde otra app ─────────────────────────────────

    private fun recoger(intent: Intent?) {
        if (intent == null || intent.action != Intent.ACTION_VIEW) return
        val datos = intent.data ?: return
        // Solo ficheros; los enlaces `proxybuilder://` son del plugin de deep links.
        if (datos.scheme != "content") return
        val uri = datos.toString()
        val canal = this.canal
        if (canal == null) {
            pendiente = uri
        } else {
            canal.send(JSObject().put("uri", uri))
        }
    }

    @Command
    fun escucharAbiertos(invoke: Invoke) {
        val args = invoke.parseArgs(EscuchaArgs::class.java)
        canal = args.handler
        val guardado = pendiente
        pendiente = null
        if (guardado != null) canal?.send(JSObject().put("uri", guardado))
        invoke.resolve()
    }

    override fun load(webView: android.webkit.WebView) {
        super.load(webView)
        // Al volver desde «Recientes» tras morir el proceso, Android entrega otra
        // vez el intent con el que se abrió: ese `.pbdeck` ya se importó.
        val intent = activity.intent
        if (intent != null &&
            (intent.flags and Intent.FLAG_ACTIVITY_LAUNCHED_FROM_HISTORY) == 0
        ) {
            recoger(intent)
        }
    }

    override fun onNewIntent(intent: Intent) {
        recoger(intent)
    }
}
