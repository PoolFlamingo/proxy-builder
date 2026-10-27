import type * as Rapier from "@dimforge/rapier3d-deterministic-compat";
import { entre, generador, girar, giroAlAzar } from "./azar";
import { caraLeida, geometria, type TipoDado, type V3 } from "./geometria";

/**
 * **La física de una tirada** con Rapier (el build determinista): se simula
 * entera antes de enseñarla, se graba cuadro a cuadro y luego la escena 3D
 * solo la reproduce. Así se sabe qué cara quedó arriba antes de pintar los
 * números (`geometria.conResultado`).
 *
 * La mesa de la física es fija (`ANCHO` × `FONDO`, en «dados»), no la del
 * tablero de cada pantalla: con la misma semilla sale la misma animación en
 * todos los equipos, y la escena la escala a lo que mida el tablero.
 */

export const ANCHO = 14;
export const FONDO = 9;
/** Cuadros por segundo de la simulación (y de la reproducción). */
export const HZ = 60;
/** Como mucho, 4 s de tirada. */
const MAX_CUADROS = 4 * HZ;
/** Cuadros seguidos casi quietos para darla por parada. */
const QUIETOS = 12;

export type Pieza = { forma: "dado"; tipo: TipoDado } | { forma: "moneda" };

export type Grabacion = {
	cuadros: number;
	/** Por pieza: posición (x, y, z) y giro (x, y, z, w) de cada cuadro. */
	piezas: { pos: Float32Array; rot: Float32Array }[];
	/** Por pieza: la cara que quedó para leer (dados) o si quedó arriba el lado A (monedas). */
	finales: { cara: number; ladoA: boolean }[];
	/** Los choques, en orden: en qué cuadro, qué pieza y con qué fuerza (0–1). Para el sonido. */
	golpes: Golpe[];
};

export type Golpe = { cuadro: number; pieza: number; fuerza: number };

/** Un cambio de velocidad mayor que este (sin la gravedad) es un choque. */
const UMBRAL_GOLPE = 1.4;
/** El cambio que suena a todo volumen. */
const GOLPE_FUERTE = 14;
/** Cuadros mínimos entre dos golpes de la misma pieza. */
const ENTRE_GOLPES = 4;

type Motor = typeof Rapier;
let cargado: Promise<Motor> | null = null;

/** Carga Rapier una vez (es WebAssembly: va en un trozo aparte). */
export function cargarRapier(): Promise<Motor> {
	cargado ??= import("@dimforge/rapier3d-deterministic-compat").then(async (m) => {
		const r = ((m as unknown as { default?: Motor }).default ?? m) as Motor;
		await r.init();
		return r;
	});
	return cargado;
}

/** Radio y medio grosor de la moneda. */
export const MONEDA = { radio: 0.5, grosor: 0.05 };

/** Lo que se lee de una pieza con este giro, y si está bien apoyada. */
function lectura(p: Pieza, rot: Rapier.Rotation) {
	const q = [rot.x, rot.y, rot.z, rot.w];
	if (p.forma === "moneda") {
		const y = girar(q, [0, 1, 0])[1];
		return { cara: 0, ladoA: y > 0, plana: Math.abs(y) };
	}
	const { cara, plana } = caraLeida(geometria(p.tipo), (v: V3) => girar(q, v));
	return { cara, ladoA: false, plana };
}

/** Simula la tirada de `piezas` con la `semilla` del evento. */
export async function simular(piezas: Pieza[], semilla: number): Promise<Grabacion> {
	const R = await cargarRapier();
	const r = generador(semilla);
	const mundo = new R.World({ x: 0, y: -40, z: 0 });
	mundo.timestep = 1 / HZ;
	try {
		// Suelo y paredes invisibles con la forma de la mesa de la física.
		const suelo = (x: number, y: number, z: number, hx: number, hy: number, hz: number) =>
			mundo.createCollider(
				R.ColliderDesc.cuboid(hx, hy, hz)
					.setTranslation(x, y, z)
					.setFriction(0.6)
					.setRestitution(0.3),
			);
		suelo(0, -0.5, 0, ANCHO / 2 + 1, 0.5, FONDO / 2 + 1);
		suelo(-ANCHO / 2 - 0.5, 3, 0, 0.5, 4, FONDO / 2 + 1);
		suelo(ANCHO / 2 + 0.5, 3, 0, 0.5, 4, FONDO / 2 + 1);
		suelo(0, 3, -FONDO / 2 - 0.5, ANCHO / 2 + 1, 4, 0.5);
		suelo(0, 3, FONDO / 2 + 0.5, ANCHO / 2 + 1, 4, 0.5);

		// Entran desde el lado del jugador (abajo, +z), en filas si son muchas.
		const porFila = 8;
		const cuerpos = piezas.map((p, i) => {
			const fila = Math.floor(i / porFila);
			const col = i % porFila;
			const enFila = Math.min(porFila, piezas.length - fila * porFila);
			const x = (col - (enFila - 1) / 2) * 1.3 + entre(r, -0.3, 0.3);
			const [qx, qy, qz, qw] = giroAlAzar(r);
			const cuerpo = mundo.createRigidBody(
				R.RigidBodyDesc.dynamic()
					.setTranslation(x, 1.2 + fila * 1.2 + entre(r, 0, 0.5), FONDO / 2 - 1)
					.setRotation({ x: qx, y: qy, z: qz, w: qw })
					.setLinvel(entre(r, -1.5, 1.5), entre(r, 2, 4), -entre(r, 3, 5))
					.setAngvel({ x: entre(r, -18, 18), y: entre(r, -18, 18), z: entre(r, -18, 18) })
					.setLinearDamping(0.5)
					.setAngularDamping(0.6)
					.setCcdEnabled(true),
			);
			const forma =
				p.forma === "moneda"
					? R.ColliderDesc.cylinder(MONEDA.grosor, MONEDA.radio)
					: R.ColliderDesc.convexHull(new Float32Array(geometria(p.tipo).vertices.flat()));
			if (!forma) throw new Error("forma");
			mundo.createCollider(forma.setDensity(1).setFriction(0.5).setRestitution(0.35), cuerpo);
			return cuerpo;
		});

		const pos = piezas.map(() => new Float32Array(MAX_CUADROS * 3));
		const rot = piezas.map(() => new Float32Array(MAX_CUADROS * 4));
		let quietos = 0;
		let empujones = 0;
		let n = 0;
		const golpes: Golpe[] = [];
		const ultimoGolpe = piezas.map(() => -ENTRE_GOLPES);
		for (; n < MAX_CUADROS; n++) {
			const antes = cuerpos.map((c) => c.linvel());
			mundo.step();
			let quieto = true;
			cuerpos.forEach((c, i) => {
				// Lo que cambió la velocidad aparte de la gravedad: un choque.
				const v0 = antes[i];
				const v1 = c.linvel();
				const dv = Math.hypot(v1.x - v0.x, v1.y - v0.y + 40 / HZ, v1.z - v0.z);
				if (dv > UMBRAL_GOLPE && n - ultimoGolpe[i] >= ENTRE_GOLPES) {
					ultimoGolpe[i] = n;
					golpes.push({ cuadro: n, pieza: i, fuerza: Math.min(1, dv / GOLPE_FUERTE) });
				}
				const t = c.translation();
				const q = c.rotation();
				pos[i].set([t.x, t.y, t.z], n * 3);
				rot[i].set([q.x, q.y, q.z, q.w], n * 4);
				const v = c.linvel();
				const w = c.angvel();
				if (v.x * v.x + v.y * v.y + v.z * v.z > 0.01 || w.x * w.x + w.y * w.y + w.z * w.z > 0.02) {
					quieto = false;
				}
			});
			quietos = quieto ? quietos + 1 : 0;
			if (quietos < QUIETOS) continue;
			// Parados: si alguno quedó de canto, un empujoncito (hasta tres veces).
			const torcidos = cuerpos.filter((c, i) => lectura(piezas[i], c.rotation()).plana < 0.94);
			if (torcidos.length === 0 || empujones >= 3) {
				n++;
				break;
			}
			empujones++;
			quietos = 0;
			for (const c of torcidos) {
				c.applyImpulse({ x: entre(r, -0.3, 0.3), y: 1.2, z: entre(r, -0.3, 0.3) }, true);
				c.applyTorqueImpulse({ x: entre(r, -0.1, 0.1), y: 0, z: entre(r, -0.1, 0.1) }, true);
			}
		}
		const cuadros = Math.min(n, MAX_CUADROS);
		return {
			cuadros,
			piezas: piezas.map((_, i) => ({
				pos: pos[i].slice(0, cuadros * 3),
				rot: rot[i].slice(0, cuadros * 4),
			})),
			finales: cuerpos.map((c, i) => {
				const l = lectura(piezas[i], c.rotation());
				return { cara: l.cara, ladoA: l.ladoA };
			}),
			golpes: golpes.filter((g) => g.cuadro < cuadros),
		};
	} finally {
		mundo.free();
	}
}
