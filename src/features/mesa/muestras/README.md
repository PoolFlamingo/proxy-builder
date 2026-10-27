# Muestras de sonido de la mesa

Cartas y dados del pack **Casino Audio** (versión 1.1) de Kenney Vleugels
([kenney.nl](https://kenney.nl/assets/casino-audio)), licencia **CC0**
(`LICENSE-Kenney.txt`): uso libre, personal y comercial; el crédito no es
obligatorio pero se agradece (está en el README del proyecto).

Se usan 23 de los 50 ficheros, convertidos del OGG original a WAV (PCM de 16
bits, estéreo, 44,1 kHz) porque `decodeAudioData` lee WAV igual en WebKitGTK,
WebView2 y WKWebView, y no lleva el desfase de arranque del MP3. Con ffmpeg, a
cada uno:

- se quitan el silencio del principio y del final
  (`silenceremove=start_periods=1:start_threshold=-65dB`, antes y después de
  `areverse`);
- los golpes de dado (`die-throw-*`, `dice-throw-2`) se cortan a 0,4 s con
  0,08 s de fundido de salida;
- se ajusta la ganancia para que el pico quede en 0,89 (`volume=`).

Qué suena con cada cosa está en `../muestras.ts` (`MUESTRAS`). Las monedas, los
sonidos de turno, chat y sparring siguen sintetizados (`../sonidos.ts`,
`../dados/sonidos.ts`).
