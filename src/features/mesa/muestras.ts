import { type Audio, audio as audioMesa, azar } from "./audio";

/**
 * **Las muestras de sonido de la mesa**: cartas (`card-*`) y dados (`die-*`,
 * `dice-*`) del pack «Casino Audio» de Kenney (CC0; ver `muestras/`). Van
 * como WAV dentro de la aplicación y se cargan solo cuando hacen falta, cada
 * una en su trozo (`?inline`: la CSP no deja a `fetch` leer ficheros propios,
 * así que se decodifican desde el propio módulo). Mientras una no está lista
 * (la primera vez, o si falla), `tocar` devuelve `false` y suena el sonido
 * sintetizado de siempre.
 */
const SLIDES = [
	"card-slide-1",
	"card-slide-2",
	"card-slide-3",
	"card-slide-4",
	"card-slide-5",
	"card-slide-6",
	"card-slide-7",
	"card-slide-8",
] as const;

export const MUESTRAS = {
	barajar: ["card-shuffle", "card-fan-1", "card-fan-2"],
	robar: SLIDES,
	girar: ["card-shove-1", "card-shove-4"],
	jugar: ["card-place-1", "card-place-2", "card-place-4"],
	mover: SLIDES,
	golpeDado: ["die-throw-1", "die-throw-3", "die-throw-4", "dice-throw-2"],
	agitarDados: ["dice-shake-1", "dice-shake-2", "dice-shake-3"],
} as const;

const CARGADORES = import.meta.glob<string>("./muestras/*.wav", {
	query: "?inline",
	import: "default",
});

const buffers = new Map<string, AudioBuffer>();
const pendientes = new Set<string>();

/** Si hay un fichero con ese nombre (para el test: nada de erratas en `MUESTRAS`). */
export const existeMuestra = (nombre: string) => `./muestras/${nombre}.wav` in CARGADORES;

/** Los bytes de un `data:` URL en base64. */
export function bytesDe(url: string): ArrayBuffer {
	const texto = atob(url.slice(url.indexOf(",") + 1));
	const bytes = new Uint8Array(texto.length);
	for (let i = 0; i < texto.length; i++) bytes[i] = texto.charCodeAt(i);
	return bytes.buffer;
}

/** Empieza a cargar y decodificar estas muestras (las que ya están o fallaron, no). */
export function precargar(ctx: AudioContext, nombres: readonly string[]) {
	for (const nombre of nombres) {
		const carga = CARGADORES[`./muestras/${nombre}.wav`];
		if (!carga || buffers.has(nombre) || pendientes.has(nombre)) continue;
		pendientes.add(nombre);
		void carga()
			.then((url) => ctx.decodeAudioData(bytesDe(url)))
			.then((b) => buffers.set(nombre, b))
			// Sin muestra suena la sintetizada: no se reintenta (`pendientes` la deja marcada).
			.catch(() => undefined);
	}
}

/** Carga las muestras de los sonidos que están encendidos al abrir la mesa. */
export function precargarMuestras() {
	const cartas = audioMesa((m) => m.sonidoCartas);
	if (cartas) {
		for (const g of ["barajar", "robar", "girar", "jugar", "mover"] as const) {
			precargar(cartas.ctx, MUESTRAS[g]);
		}
	}
	const dados = audioMesa((m) => m.sonidoDados);
	if (dados) {
		precargar(dados.ctx, MUESTRAS.golpeDado);
		precargar(dados.ctx, MUESTRAS.agitarDados);
	}
}

/**
 * Toca una de estas muestras (al azar entre las que ya están cargadas) en el
 * instante `t`, con un tono que varía un poco. `desde` y `dura` tocan solo un
 * trozo, con la cola apagándose. `false` si ninguna está lista todavía.
 */
export function tocar(
	a: Audio,
	nombres: readonly string[],
	o: { t: number; volumen: number; tono?: number; desde?: number; dura?: number },
): boolean {
	precargar(a.ctx, nombres);
	const listas = nombres.filter((n) => buffers.has(n));
	const buffer = buffers.get(listas[Math.floor(Math.random() * listas.length)] ?? "");
	if (!buffer) return false;
	const fuente = a.ctx.createBufferSource();
	fuente.buffer = buffer;
	fuente.playbackRate.value = o.tono ?? azar(0.96, 1.05);
	const ganancia = a.ctx.createGain();
	ganancia.gain.setValueAtTime(o.volumen, o.t);
	if (o.dura !== undefined) {
		ganancia.gain.setValueAtTime(o.volumen, o.t + Math.max(0, o.dura - 0.08));
		ganancia.gain.linearRampToValueAtTime(0, o.t + o.dura);
	}
	fuente.connect(ganancia).connect(a.salida);
	fuente.start(o.t, o.desde ?? 0, o.dura);
	return true;
}
