import { claveEstilo, type Estilo } from "./aspecto";
import { entre, generador } from "./azar";

/**
 * **El fondo de las caras**: el color con su acabado (nacarado, mármol,
 * purpurina) o la imagen propia, pintado una vez en un lienzo cuadrado. El
 * 3D lo usa de textura (un trozo distinto en cada cara) y el CSS 3D y el 2D
 * como imagen de fondo. Los acabados son dibujo con azar de semilla fija:
 * el mismo color da siempre el mismo dado.
 */

export const LADO_FONDO = 256;

const imagenes = new Map<string, Promise<HTMLImageElement>>();
function cargarImagen(url: string): Promise<HTMLImageElement> {
	let p = imagenes.get(url);
	if (!p) {
		p = new Promise((listo, fallo) => {
			const img = new Image();
			img.onload = () => listo(img);
			img.onerror = fallo;
			img.src = url;
		});
		imagenes.set(url, p);
	}
	return p;
}

function semillaDe(texto: string): number {
	let h = 0x811c9dc5;
	for (let i = 0; i < texto.length; i++) h = Math.imul(h ^ texto.charCodeAt(i), 0x01000193);
	return h >>> 0;
}

function nacarado(ctx: CanvasRenderingContext2D, lado: number, r: () => number) {
	// Un velo de colores de nácar (rosa, celeste, crema, lila) sobre el color,
	// a poca opacidad para que se note igual sobre claro que sobre oscuro.
	const g = ctx.createLinearGradient(0, 0, lado, lado);
	g.addColorStop(0, "rgba(255,150,215,.32)");
	g.addColorStop(0.35, "rgba(130,215,255,.3)");
	g.addColorStop(0.65, "rgba(255,240,170,.3)");
	g.addColorStop(1, "rgba(190,150,255,.32)");
	ctx.fillStyle = g;
	ctx.fillRect(0, 0, lado, lado);
	// Ondas suaves, como el nácar: una clara y otra tenue y oscura al lado.
	for (let k = 0; k < 7; k++) {
		const y0 = entre(r, 0, lado);
		const amp = entre(r, lado * 0.04, lado * 0.12);
		const fase = entre(r, 0, Math.PI * 2);
		const ancho = entre(r, lado * 0.03, lado * 0.07);
		for (const [dy, tono] of [
			[0, `rgba(255,255,255,${entre(r, 0.2, 0.35)})`],
			[ancho, "rgba(80,40,120,.1)"],
		] as const) {
			ctx.beginPath();
			for (let x = 0; x <= lado; x += 4) {
				const y = y0 + dy + Math.sin((x / lado) * Math.PI * 2 + fase) * amp;
				if (x === 0) ctx.moveTo(x, y);
				else ctx.lineTo(x, y);
			}
			ctx.strokeStyle = tono;
			ctx.lineWidth = ancho;
			ctx.stroke();
		}
	}
	const b = ctx.createRadialGradient(
		lado * 0.3,
		lado * 0.25,
		0,
		lado * 0.3,
		lado * 0.25,
		lado * 0.7,
	);
	b.addColorStop(0, "rgba(255,255,255,.3)");
	b.addColorStop(1, "rgba(255,255,255,0)");
	ctx.fillStyle = b;
	ctx.fillRect(0, 0, lado, lado);
}

function marmol(ctx: CanvasRenderingContext2D, lado: number, r: () => number, claro: boolean) {
	// Nubes de fondo.
	for (let k = 0; k < 14; k++) {
		const x = entre(r, 0, lado);
		const y = entre(r, 0, lado);
		const rad = entre(r, lado * 0.15, lado * 0.45);
		const g = ctx.createRadialGradient(x, y, 0, x, y, rad);
		const tono = r() < 0.5 ? "255,255,255" : "0,0,0";
		g.addColorStop(0, `rgba(${tono},${entre(r, 0.05, 0.12)})`);
		g.addColorStop(1, `rgba(${tono},0)`);
		ctx.fillStyle = g;
		ctx.fillRect(0, 0, lado, lado);
	}
	// Vetas: un camino que serpentea, en tres pasadas (ancha y tenue a fina y
	// marcada) para que parezca difuminado sin `filter` (WebKitGTK no lo tiene).
	const veta = claro ? "40,30,30" : "255,255,255";
	for (let k = 0; k < 6; k++) {
		const puntos: [number, number][] = [];
		let x = -10;
		let y = entre(r, 0, lado);
		const pendiente = entre(r, -0.6, 0.6);
		while (x < lado + 10) {
			puntos.push([x, y]);
			x += lado / 20;
			y += pendiente * (lado / 20) + entre(r, -1, 1) * (lado / 16);
		}
		for (const [ancho, alfa] of [
			[lado * 0.05, 0.06],
			[lado * 0.022, 0.14],
			[lado * 0.007, 0.45],
		] as const) {
			ctx.beginPath();
			puntos.forEach(([px, py], i) => {
				if (i === 0) ctx.moveTo(px, py);
				else ctx.lineTo(px, py);
			});
			ctx.strokeStyle = `rgba(${veta},${alfa * (k < 2 ? 1 : 0.6)})`;
			ctx.lineWidth = ancho * (k < 2 ? 1 : 0.6);
			ctx.lineJoin = "round";
			ctx.lineCap = "round";
			ctx.stroke();
		}
	}
}

function purpurina(ctx: CanvasRenderingContext2D, lado: number, r: () => number) {
	const n = Math.round((lado * lado) / 36);
	for (let k = 0; k < n; k++) {
		const x = entre(r, 0, lado);
		const y = entre(r, 0, lado);
		const t = entre(r, 0.5, 1.6);
		const brillo = r();
		ctx.fillStyle =
			brillo < 0.75
				? `rgba(255,255,255,${entre(r, 0.12, 0.6)})`
				: `hsla(${Math.floor(entre(r, 0, 360))},90%,75%,${entre(r, 0.3, 0.7)})`;
		ctx.fillRect(x, y, t, t);
	}
	// Destellos.
	for (let k = 0; k < 14; k++) {
		const x = entre(r, 0, lado);
		const y = entre(r, 0, lado);
		const t = entre(r, lado * 0.015, lado * 0.035);
		const g = ctx.createRadialGradient(x, y, 0, x, y, t * 2.2);
		g.addColorStop(0, "rgba(255,255,255,.9)");
		g.addColorStop(1, "rgba(255,255,255,0)");
		ctx.fillStyle = g;
		ctx.fillRect(x - t * 2.2, y - t * 2.2, t * 4.4, t * 4.4);
		ctx.fillStyle = "rgba(255,255,255,.85)";
		ctx.fillRect(x - t * 2, y - 0.5, t * 4, 1);
		ctx.fillRect(x - 0.5, y - t * 2, 1, t * 4);
	}
}

const lienzos = new Map<string, Promise<HTMLCanvasElement>>();

/** El fondo de las caras de un estilo (se guarda: el mismo estilo no se repinta). */
export function lienzoFondo(e: Estilo): Promise<HTMLCanvasElement> {
	const clave = claveEstilo(e);
	let p = lienzos.get(clave);
	if (!p) {
		p = pintarFondo(e);
		lienzos.set(clave, p);
		// Sin crecer sin fin si se prueban muchos colores.
		if (lienzos.size > 24) {
			const primera = lienzos.keys().next().value;
			if (primera !== undefined) lienzos.delete(primera);
		}
	}
	return p;
}

async function pintarFondo(e: Estilo): Promise<HTMLCanvasElement> {
	const lado = LADO_FONDO;
	const c = document.createElement("canvas");
	c.width = c.height = lado;
	const ctx = c.getContext("2d");
	if (!ctx) return c;
	ctx.fillStyle = e.color;
	ctx.fillRect(0, 0, lado, lado);
	if (e.imagen) {
		try {
			ctx.drawImage(await cargarImagen(e.imagen), 0, 0, lado, lado);
			return c;
		} catch {
			// Sin la imagen, el color.
		}
	}
	const r = generador(semillaDe(`${e.color}|${e.acabado}`));
	ctx.save();
	if (e.acabado === "nacarado") nacarado(ctx, lado, r);
	else if (e.acabado === "marmol") {
		const n = Number.parseInt(e.color.slice(1, 7), 16);
		const luz = 0.2126 * ((n >> 16) & 255) + 0.7152 * ((n >> 8) & 255) + 0.0722 * (n & 255);
		marmol(ctx, lado, r, luz > 170);
	} else if (e.acabado === "purpurina") purpurina(ctx, lado, r);
	ctx.restore();
	return c;
}

const urls = new Map<HTMLCanvasElement, string>();

/** El fondo como `data:` URL, para el CSS 3D y el 2D. */
export function urlFondo(c: HTMLCanvasElement): string {
	let u = urls.get(c);
	if (!u) {
		u = c.toDataURL("image/jpeg", 0.9);
		urls.set(c, u);
	}
	return u;
}

/**
 * El trozo del fondo de la cara `i` (x, y y lado, en px del fondo): con un
 * acabado, un trozo distinto en cada cara para que no se repitan; con la
 * imagen propia, entera.
 */
export function trozoDeCara(e: Estilo, i: number): [number, number, number] {
	if (e.imagen || e.acabado === "liso" || e.acabado === "translucido") return [0, 0, LADO_FONDO];
	const r = generador(0x51ed ^ (i * 2654435761));
	const lado = LADO_FONDO / 2;
	return [Math.floor(r() * lado), Math.floor(r() * lado), lado];
}
