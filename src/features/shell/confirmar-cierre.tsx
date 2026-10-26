import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { Confirmar } from "@/components/confirmar";
import { useLang } from "@/components/language-provider";
import { cancelarTrabajo } from "@/features/decklist/acciones";
import { guardar as guardarPartida } from "@/features/mesa/conexion";
import { hayPendientes, type Pendientes, pendientes } from "@/lib/cierre";
import { guardarAhora } from "@/stores/deck-store";
import { useEdicion } from "@/stores/edicion-store";

/** Guarda lo que quede del deck (el autoguardado espera 800 ms); `false` si no pudo. */
async function guardarElDeck(): Promise<boolean> {
	try {
		await guardarAhora();
		return true;
	} catch {
		return false;
	}
}

/**
 * **Cerrar la ventana**: si hay un trabajo largo en marcha, una actualización
 * bajando, algo sin guardar en un diálogo o en la carta que se diseña, se
 * frena el cierre y se pregunta; sin nada de eso, se guarda el deck (por si el
 * último cambio no llegó al autoguardado) y se cierra.
 *
 * Con la carta sin guardar hay una salida más: «Guardar y cerrar».
 *
 * 🛑 Con un escuchante de `onCloseRequested`, Tauri ya no cierra por su
 * cuenta: lo hace `destroy()` (permiso `core:window:allow-destroy` en
 * `capabilities/default.json`; sin él la ventana no se podría cerrar).
 */
export function ConfirmarCierre() {
	const { t } = useLang();
	const [pend, setPend] = useState<Pendientes | null>(null);
	// El diálogo está abierto: otro intento de cerrar no hace nada más.
	const abierto = useRef(false);

	useEffect(() => {
		let quitar: (() => void) | null = null;
		let vivo = true;
		try {
			void getCurrentWindow()
				.onCloseRequested(async (e) => {
					if (abierto.current) {
						e.preventDefault();
						return;
					}
					const guardado = await guardarElDeck();
					await guardarPartida();
					const p = pendientes(!guardado);
					if (!hayPendientes(p)) return;
					e.preventDefault();
					abierto.current = true;
					setPend(p);
				})
				.then((dejar) => {
					if (vivo) quitar = dejar;
					else dejar();
				})
				.catch(() => {});
		} catch {
			// Fuera de Tauri (la vista en un navegador): no hay ventana que cerrar.
		}
		return () => {
			vivo = false;
			quitar?.();
		};
	}, []);

	const cerrarDialogo = () => {
		abierto.current = false;
		setPend(null);
	};

	const cerrar = async () => {
		if (pend?.trabajo != null) cancelarTrabajo();
		await guardarElDeck();
		await guardarPartida();
		try {
			await getCurrentWindow().destroy();
		} catch {
			toast.error(t("No se ha podido cerrar la ventana."));
		}
	};

	const razones: string[] = [];
	if (pend?.edicion) razones.push(t("Hay cambios sin guardar en la carta que estás diseñando."));
	if (pend?.trabajo != null) {
		razones.push(
			pend.trabajo
				? t("Hay un trabajo en marcha ({fase}).", { fase: pend.trabajo })
				: t("Hay un trabajo en marcha."),
		);
	}
	if (pend?.actualizando) razones.push(t("Se está descargando o instalando una actualización."));
	if (pend?.dialogoEnCurso) razones.push(t("Hay un proceso en marcha en una ventana abierta."));
	if (pend?.dialogoSinGuardar) razones.push(t("Hay cambios sin guardar en una ventana abierta."));
	if (pend?.partida)
		razones.push(t("Hay una partida en la mesa: se guarda y podrás seguirla al volver."));
	if (pend?.deck)
		razones.push(t("No se ha podido guardar el deck: se perderán sus últimos cambios."));

	return (
		<Confirmar
			abierto={pend !== null}
			onCerrar={cerrarDialogo}
			titulo={t("¿Cerrar Proxy Builder?")}
			descripcion={t("Si cierras ahora, esto se pierde o se interrumpe:")}
			accion={t("Cerrar igualmente")}
			destructiva
			onConfirmar={() => void cerrar()}
			accionAlternativa={
				pend?.edicion
					? {
							texto: t("Guardar y cerrar"),
							onClick: async () => {
								const ok = (await useEdicion.getState().pendiente?.guardar()) ?? true;
								if (ok) await cerrar();
								return ok;
							},
						}
					: undefined
			}
		>
			<ul className="list-disc space-y-1 pl-5 text-xs text-muted-foreground">
				{razones.map((r) => (
					<li key={r}>{r}</li>
				))}
			</ul>
		</Confirmar>
	);
}
