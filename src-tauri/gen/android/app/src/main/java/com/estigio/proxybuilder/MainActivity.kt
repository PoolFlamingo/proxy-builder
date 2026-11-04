package com.estigio.proxybuilder

import android.graphics.Color
import android.os.Bundle
import android.view.View
import androidx.activity.enableEdgeToEdge
import androidx.core.graphics.Insets
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat

/**
 * La app va siempre a pantalla completa: sin barra de estado ni de navegación
 * (un gesto desde el borde las enseña un momento). Se vuelve a pedir cada vez
 * que la ventana recupera el foco, porque un diálogo del sistema, el teclado o
 * volver de otra app las devuelven.
 *
 * La muesca no la resuelve la web: el hueco de la cámara se deja fuera con un
 * margen en la vista que contiene el WebView, y al WebView le llega sin ella
 * (así sus `env(safe-area-inset-*)` valen 0 y no se cuenta dos veces). El
 * teclado se deja fuera igual: de borde a borde (obligatorio desde Android 15)
 * el sistema ya no encoge la ventana y, sin este margen, el teclado tapaba lo
 * que se escribe sin que el WebView se enterara. Con él encogen la ventana y el
 * `visualViewport` (`lib/teclado.ts`).
 */
class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    window.decorView.setBackgroundColor(Color.BLACK)
    val contenido = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(contenido) { vista, insets ->
      val muesca = insets.getInsets(WindowInsetsCompat.Type.displayCutout())
      val teclado = insets.getInsets(WindowInsetsCompat.Type.ime())
      vista.setPadding(muesca.left, muesca.top, muesca.right, maxOf(muesca.bottom, teclado.bottom))
      WindowInsetsCompat.Builder(insets)
        .setInsets(WindowInsetsCompat.Type.displayCutout(), Insets.NONE)
        .setInsets(WindowInsetsCompat.Type.systemBars(), Insets.NONE)
        .setInsets(WindowInsetsCompat.Type.ime(), Insets.NONE)
        .setDisplayCutout(null)
        .build()
    }
    sinBarras()
  }

  override fun onResume() {
    super.onResume()
    sinBarras()
  }

  override fun onWindowFocusChanged(hasFocus: Boolean) {
    super.onWindowFocusChanged(hasFocus)
    if (hasFocus) sinBarras()
  }

  private fun sinBarras() {
    val control = WindowCompat.getInsetsController(window, window.decorView)
    control.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
    control.hide(WindowInsetsCompat.Type.systemBars())
  }
}
