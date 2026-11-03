/**
 * **Leer un código QR con la cámara** (solo móvil). La única puerta al plugin
 * `tauri-plugin-barcode-scanner`, como `dialogos.ts` lo es del de los
 * diálogos: nadie más lo importa. En escritorio no hay cámara que valga para
 * esto —«Recibir» enseña el QR y el texto, y en el ordenador se pega el
 * texto—, así que `escanerDisponible()` es `false` y no se carga nada.
 */
import { esMovil } from "@/lib/plataforma";

export type ResultadoEscaneo =
	/** Se leyó un QR: su texto. */
	| { tipo: "leido"; texto: string }
	/** La persona cerró el lector sin leer nada. */
	| { tipo: "cancelado" }
	/** No hay permiso de la cámara (y no se puede pedir otra vez desde aquí). */
	| { tipo: "sin-permiso" };

/** ¿Se puede escanear? Solo en Android e iOS. */
export function escanerDisponible(): Promise<boolean> {
	return esMovil();
}

/** Pide la cámara si hace falta. `true`: se puede usar. */
async function permiso(): Promise<boolean> {
	const { checkPermissions, requestPermissions } = await import(
		"@tauri-apps/plugin-barcode-scanner"
	);
	let estado: string = await checkPermissions();
	if (estado === "prompt" || estado === "prompt-with-rationale") {
		estado = await requestPermissions();
	}
	return estado === "granted";
}

/** Abre el lector de la cámara y espera un QR. */
export async function escanearQr(): Promise<ResultadoEscaneo> {
	if (!(await esMovil())) return { tipo: "cancelado" };
	const { scan, Format } = await import("@tauri-apps/plugin-barcode-scanner");
	if (!(await permiso())) return { tipo: "sin-permiso" };
	try {
		const r = await scan({ formats: [Format.QRCode], windowed: false });
		return r.content ? { tipo: "leido", texto: r.content } : { tipo: "cancelado" };
	} catch {
		// Cerrar el lector (o el «atrás») rechaza la promesa.
		return { tipo: "cancelado" };
	}
}

/** Los ajustes de la app en el móvil, para dar el permiso de la cámara a mano. */
export async function abrirAjustesDeLaCamara(): Promise<void> {
	if (!(await esMovil())) return;
	try {
		const { openAppSettings } = await import("@tauri-apps/plugin-barcode-scanner");
		await openAppSettings();
	} catch {
		// Sin ajustes que abrir.
	}
}

/** Cierra el lector si está abierto (al cerrar el diálogo, por ejemplo). */
export async function cancelarEscaneo(): Promise<void> {
	if (!(await esMovil())) return;
	try {
		const { cancel } = await import("@tauri-apps/plugin-barcode-scanner");
		await cancel();
	} catch {
		// No había nada abierto.
	}
}
