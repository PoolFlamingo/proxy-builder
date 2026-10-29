import * as THREE from "three";
import { type Estilo, METALES } from "./aspecto";
import type { PiezaTirada, Tirada } from "./cola";
import { ANCHO, FONDO, type Grabacion, HZ, MONEDA, simular } from "./fisica";
import { angulosEsquinas, esquinasD4, geometria, planoCara, textoCara } from "./geometria";
import { tipoDe, valoresPintados } from "./piezas";
import { CAOS, CARA, CRUZ, PLANESWALK } from "./simbolos";
import { LADO_FONDO, trozoDeCara } from "./texturas";

/**
 * **Los dados en 3D** (three.js + Rapier): una escena transparente encima
 * del tablero. Cada tirada se simula entera (`fisica.ts`), se pintan los
 * números con el resultado del motor en la cara que quedó arriba y se
 * reproduce la grabación. Este módulo se carga aparte (`import()`), solo al
 * tirar con el 3D activo. El aspecto (`Estilo`) pone el fondo de las caras,
 * el color de los números, el material y el metal de la moneda.
 */

/** Cómo se pintan los números y símbolos. */
type Tinta = { relleno: string; trazo: string };

/** Lo que suena: cada golpe de la física, cuando ocurre. */
export type AlGolpe = (moneda: boolean, fuerza: number) => void;

/** Píxeles de la textura por cara. */
const CELDA = 128;

type Pintable =
	| { texto: string }
	| { simbolo: string }
	/** `angulos`: dónde está cada esquina, desde la primera (que va arriba). */
	| { esquinas: string[]; angulos?: number[] }
	| null;

/** Pinta lo de una cara en el centro de una celda (con «arriba» hacia +y de la cara). */
function pintar(
	ctx: CanvasRenderingContext2D,
	cx: number,
	cy: number,
	r: number,
	cosa: Pintable,
	pequeno: boolean,
	tinta: Tinta,
) {
	if (!cosa) return;
	ctx.save();
	ctx.translate(cx, cy);
	ctx.fillStyle = tinta.relleno;
	ctx.strokeStyle = tinta.trazo;
	ctx.lineJoin = "round";
	if ("texto" in cosa) {
		const t = cosa.texto;
		const tam = r * (pequeno ? 0.62 : 0.85) * (t.length > 2 ? 0.75 : 1);
		ctx.font = `700 ${tam}px system-ui, sans-serif`;
		ctx.textAlign = "center";
		ctx.textBaseline = "middle";
		ctx.lineWidth = tam * 0.12;
		ctx.strokeText(t, 0, tam * 0.04);
		ctx.fillText(t, 0, tam * 0.04);
	} else if ("simbolo" in cosa) {
		const k = (r * 1.2) / 100;
		ctx.scale(k, k);
		ctx.translate(-50, -50);
		const p = new Path2D(cosa.simbolo);
		ctx.lineWidth = 6;
		ctx.stroke(p);
		ctx.fill(p);
	} else {
		// d4: un número junto a cada esquina, mirando hacia ella.
		cosa.esquinas.forEach((t, i) => {
			const a = cosa.angulos?.[i] ?? 0;
			ctx.save();
			ctx.rotate(a);
			const tam = r * 0.38;
			ctx.font = `700 ${tam}px system-ui, sans-serif`;
			ctx.textAlign = "center";
			ctx.textBaseline = "middle";
			ctx.lineWidth = tam * 0.12;
			ctx.strokeText(t, 0, -r * 0.48);
			ctx.fillText(t, 0, -r * 0.48);
			ctx.restore();
		});
	}
	ctx.restore();
}

/** Lo que va en la cara `i` de una pieza. */
function contenido(p: PiezaTirada, valores: number[], i: number): Pintable {
	if (p.forma === "planar") {
		return valores[i] === 1 ? { simbolo: PLANESWALK } : valores[i] === 6 ? { simbolo: CAOS } : null;
	}
	if (p.forma !== "dado") return null;
	if (p.tipo === 4) {
		return { esquinas: esquinasD4(geometria(4), valores, i).map((v) => String(v)) };
	}
	return { texto: textoCara(p.tipo, valores[i], p.decenas) };
}

/** El material de cada acabado (el color y el dibujo van en la textura). */
function material(e: Estilo, tex: THREE.Texture): THREE.Material {
	const comun = { map: tex, flatShading: true };
	switch (e.imagen ? "liso" : e.acabado) {
		case "nacarado":
			return new THREE.MeshStandardMaterial({ ...comun, roughness: 0.25, metalness: 0.2 });
		case "marmol":
			return new THREE.MeshStandardMaterial({ ...comun, roughness: 0.2, metalness: 0.05 });
		case "purpurina":
			return new THREE.MeshStandardMaterial({ ...comun, roughness: 0.3, metalness: 0.45 });
		case "translucido":
			return new THREE.MeshStandardMaterial({
				...comun,
				roughness: 0.15,
				metalness: 0,
				transparent: true,
				opacity: 0.78,
			});
		default:
			return new THREE.MeshStandardMaterial({ ...comun, roughness: 0.45, metalness: 0.05 });
	}
}

/** La malla de un dado: cada cara con su trozo de textura (números ya cambiados). */
function mallaDado(p: PiezaTirada, cara: number, e: Estilo, fondo: HTMLCanvasElement): THREE.Mesh {
	const tinta = { relleno: e.numeros, trazo: e.contorno };
	const t = tipoDe(p) ?? 6;
	const g = geometria(t);
	const valores = valoresPintados(p, cara);
	const n = g.caras.length;
	const cols = Math.ceil(Math.sqrt(n));
	const filas = Math.ceil(n / cols);
	const lienzo = document.createElement("canvas");
	lienzo.width = cols * CELDA;
	lienzo.height = filas * CELDA;
	const ctx = lienzo.getContext("2d");
	const pos: number[] = [];
	const uv: number[] = [];
	if (ctx) {
		ctx.fillStyle = e.color;
		ctx.fillRect(0, 0, lienzo.width, lienzo.height);
	}
	for (let i = 0; i < n; i++) {
		const col = i % cols;
		const fila = Math.floor(i / cols);
		const pl = planoCara(g, i);
		const k = 0.46 / pl.radio;
		const aUv = ([x, y]: [number, number]) => [
			(col + 0.5 + x * k) / cols,
			1 - (fila + 0.5 - y * k) / filas,
		];
		const vs = g.caras[i];
		// En abanico desde el primer vértice.
		for (let j = 1; j + 1 < vs.length; j++) {
			for (const idx of [0, j, j + 1]) {
				pos.push(...g.vertices[vs[idx]]);
				uv.push(...aUv(pl.puntos[idx]));
			}
		}
		if (ctx) {
			// En el d4 las esquinas van hacia cada vértice: se gira la celda para que
			// la primera esquina quede arriba del todo.
			const cx = (col + 0.5) * CELDA;
			const cy = (fila + 0.5) * CELDA;
			const [fx, fy, fl] = trozoDeCara(e, i);
			ctx.drawImage(
				fondo,
				fx,
				fy,
				Math.min(fl, LADO_FONDO),
				Math.min(fl, LADO_FONDO),
				col * CELDA,
				fila * CELDA,
				CELDA,
				CELDA,
			);
			const cosa = contenido(p, valores, i);
			if (cosa && "esquinas" in cosa) {
				const [x0, y0] = pl.puntos[0];
				ctx.save();
				ctx.translate(cx, cy);
				ctx.rotate(Math.atan2(x0, y0));
				pintar(
					ctx,
					0,
					0,
					CELDA * 0.5 * (pl.radio * k * 2),
					{ ...cosa, angulos: angulosEsquinas(pl.puntos) },
					false,
					tinta,
				);
				ctx.restore();
			} else {
				pintar(ctx, cx, cy, CELDA * 0.5, cosa, t >= 12 || t === 4 || t === 8, tinta);
			}
		}
	}
	const geo = new THREE.BufferGeometry();
	geo.setAttribute("position", new THREE.Float32BufferAttribute(pos, 3));
	geo.setAttribute("uv", new THREE.Float32BufferAttribute(uv, 2));
	geo.computeVertexNormals();
	const tex = new THREE.CanvasTexture(lienzo);
	tex.colorSpace = THREE.SRGBColorSpace;
	tex.anisotropy = 4;
	const m = new THREE.Mesh(geo, material(e, tex));
	m.castShadow = true;
	return m;
}

/** La moneda: un cilindro con la corona y la cruz; arriba, lo que diga el motor. */
function mallaMoneda(cara: boolean, ladoAArriba: boolean, e: Estilo): THREE.Mesh {
	const metal = METALES[e.moneda];
	const tapa = (simbolo: string) => {
		const c = document.createElement("canvas");
		c.width = c.height = CELDA;
		const ctx = c.getContext("2d");
		if (ctx) {
			const grad = ctx.createRadialGradient(
				CELDA / 2,
				CELDA / 2,
				4,
				CELDA / 2,
				CELDA / 2,
				CELDA / 2,
			);
			grad.addColorStop(0, metal.claro);
			grad.addColorStop(1, metal.oscuro);
			ctx.fillStyle = grad;
			ctx.fillRect(0, 0, CELDA, CELDA);
			pintar(ctx, CELDA / 2, CELDA / 2, CELDA * 0.42, { simbolo }, false, {
				relleno: metal.simbolo,
				trazo: metal.trazo,
			});
		}
		const t = new THREE.CanvasTexture(c);
		t.colorSpace = THREE.SRGBColorSpace;
		return new THREE.MeshStandardMaterial({ map: t, roughness: 0.35, metalness: 0.6 });
	};
	// La tapa de arriba del cilindro es el lado A: lleva el resultado si quedó arriba.
	const arriba = ladoAArriba === cara ? CARA : CRUZ;
	const abajo = arriba === CARA ? CRUZ : CARA;
	const canto = new THREE.MeshStandardMaterial({
		color: metal.canto,
		roughness: 0.4,
		metalness: 0.6,
	});
	const m = new THREE.Mesh(
		new THREE.CylinderGeometry(MONEDA.radio, MONEDA.radio, MONEDA.grosor * 2, 40),
		[canto, tapa(arriba), tapa(abajo)],
	);
	m.castShadow = true;
	return m;
}

export class EscenaDados {
	private renderer: THREE.WebGLRenderer;
	private escena = new THREE.Scene();
	private camara = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 100);
	private piezas: THREE.Mesh[] = [];
	private pedido = 0;

	constructor(private lienzo: HTMLCanvasElement) {
		this.renderer = new THREE.WebGLRenderer({ canvas: lienzo, alpha: true, antialias: true });
		this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
		this.renderer.shadowMap.enabled = true;
		this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
		this.escena.add(new THREE.AmbientLight(0xffffff, 1.4));
		const sol = new THREE.DirectionalLight(0xffffff, 2.2);
		sol.position.set(4, 14, 6);
		sol.castShadow = true;
		sol.shadow.mapSize.set(1024, 1024);
		Object.assign(sol.shadow.camera, {
			left: -ANCHO / 2,
			right: ANCHO / 2,
			top: FONDO / 2,
			bottom: -FONDO / 2,
		});
		this.escena.add(sol);
		const suelo = new THREE.Mesh(
			new THREE.PlaneGeometry(ANCHO + 4, FONDO + 4),
			new THREE.ShadowMaterial({ opacity: 0.35 }),
		);
		suelo.rotation.x = -Math.PI / 2;
		suelo.receiveShadow = true;
		this.escena.add(suelo);
		this.camara.position.set(0, 14, 8);
		this.camara.lookAt(0, 0, 0.3);
		this.medir();
	}

	/** Ajusta la cámara para que la mesa de la física llene el lienzo. */
	medir() {
		const w = this.lienzo.clientWidth || 1;
		const h = this.lienzo.clientHeight || 1;
		this.renderer.setSize(w, h, false);
		const aspecto = w / h;
		// Lo que ocupa la mesa vista desde la cámara inclinada.
		const inclinacion = Math.cos(Math.atan2(8, 14));
		const medioAlto = Math.max((FONDO * inclinacion) / 2 + 0.6, (ANCHO / 2 + 0.6) / aspecto);
		const medioAncho = medioAlto * aspecto;
		Object.assign(this.camara, {
			left: -medioAncho,
			right: medioAncho,
			top: medioAlto,
			bottom: -medioAlto,
		});
		this.camara.updateProjectionMatrix();
	}

	private limpiar() {
		for (const m of this.piezas) {
			this.escena.remove(m);
			m.geometry.dispose();
			for (const mat of Array.isArray(m.material) ? m.material : [m.material]) {
				(mat as THREE.MeshStandardMaterial).map?.dispose();
				mat.dispose();
			}
		}
		this.piezas = [];
	}

	/** Simula y reproduce una tirada; se resuelve al pararse los dados. */
	async tirar(
		t: Tirada,
		estilo: Estilo,
		fondo: HTMLCanvasElement,
		alGolpe?: AlGolpe,
	): Promise<void> {
		const pedido = ++this.pedido;
		const grabacion: Grabacion = await simular(
			t.piezas.map((p) =>
				p.forma === "moneda" ? { forma: "moneda" } : { forma: "dado", tipo: tipoDe(p) ?? 6 },
			),
			t.semilla,
		);
		if (pedido !== this.pedido) return;
		this.limpiar();
		this.piezas = t.piezas.map((p, i) => {
			const f = grabacion.finales[i];
			const m =
				p.forma === "moneda"
					? mallaMoneda(p.cara, f.ladoA, estilo)
					: mallaDado(p, f.cara, estilo, fondo);
			this.escena.add(m);
			return m;
		});
		this.medir();
		await new Promise<void>((listo) => {
			const inicio = performance.now();
			let sonados = 0;
			const cuadro = () => {
				if (pedido !== this.pedido) return listo();
				const k = Math.min(
					grabacion.cuadros - 1,
					Math.floor(((performance.now() - inicio) / 1000) * HZ),
				);
				// Los golpes de la física hasta este cuadro, al sonido.
				while (sonados < grabacion.golpes.length && grabacion.golpes[sonados].cuadro <= k) {
					const g = grabacion.golpes[sonados++];
					alGolpe?.(t.piezas[g.pieza]?.forma === "moneda", g.fuerza);
				}
				grabacion.piezas.forEach((g, i) => {
					const m = this.piezas[i];
					m.position.set(g.pos[k * 3], g.pos[k * 3 + 1], g.pos[k * 3 + 2]);
					m.quaternion.set(g.rot[k * 4], g.rot[k * 4 + 1], g.rot[k * 4 + 2], g.rot[k * 4 + 3]);
				});
				this.renderer.render(this.escena, this.camara);
				if (k >= grabacion.cuadros - 1) listo();
				else requestAnimationFrame(cuadro);
			};
			requestAnimationFrame(cuadro);
		});
	}

	/** Quita las piezas de la mesa. */
	recoger() {
		this.pedido++;
		this.limpiar();
		this.renderer.render(this.escena, this.camara);
	}

	destruir() {
		this.recoger();
		this.renderer.dispose();
	}
}
