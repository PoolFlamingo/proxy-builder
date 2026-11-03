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
 * teclado (`ime`) sí le llega: con él encoge el `visualViewport`
 * (`lib/teclado.ts`).
 */
class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    window.decorView.setBackgroundColor(Color.BLACK)
    val contenido = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(contenido) { vista, insets ->
      val muesca = insets.getInsets(WindowInsetsCompat.Type.displayCutout())
      vista.setPadding(muesca.left, muesca.top, muesca.right, muesca.bottom)
      WindowInsetsCompat.Builder(insets)
        .setInsets(WindowInsetsCompat.Type.displayCutout(), Insets.NONE)
        .setInsets(WindowInsetsCompat.Type.systemBars(), Insets.NONE)
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
