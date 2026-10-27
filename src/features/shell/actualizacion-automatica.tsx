import { ArrowUpCircle, Loader2 } from "lucide-react";
import { useEffect, useRef } from "react";
import { toast } from "sonner";
import type { ActualizacionInfo } from "@/bindings/ActualizacionInfo";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { cn } from "@/lib/utils";
import { useActualizacion } from "@/stores/actualizacion-store";
import { useMesa } from "@/stores/mesa-store";
import { useUi } from "@/stores/ui-store";

/** La primera búsqueda espera a que la app haya arrancado del todo. */
const PRIMERA_MS = 8_000;
const CADA_MS = 6 * 60 * 60 * 1000;

/**
 * Si está encendido en Ajustes (lo está de fábrica): busca una versión nueva
 * al arrancar y cada 6 horas y la baja sin molestar (los avisos, en
 * `useAvisosActualizacion`). Sin red o sin release no dice nada.
 */
export function useActualizacionAutomatica() {
	const lang = useLang();
	const textos = useRef(lang);
	textos.current = lang;
	const activa = useUi((s) => s.actualizarSola);

	useEffect(() => {
		if (!activa) return;
		let viva = true;
		const ronda = async () => {
			const a = useActualizacion.getState();
			if (!(await a.iniciar()) || !viva) return;
			const info = await a.buscar();
			if (!info || !viva) return;
			// Los avisos los pone `useAvisosActualizacion`.
			await useActualizacion.getState().bajar();
		};
		const primera = setTimeout(ronda, PRIMERA_MS);
		const cada = setInterval(ronda, CADA_MS);
		return () => {
			viva = false;
			clearTimeout(primera);
			clearInterval(cada);
		};
	}, [activa]);
}

/**
 * Los avisos de la actualización, se baje sola o desde Ajustes: uno mientras
 * se descarga (con el porcentaje) que pasa a «lista, reiniciar» al terminar,
 * y otro si la descarga falla. Va montado una vez en `App`.
 */
export function useAvisosActualizacion() {
	const lang = useLang();
	const textos = useRef(lang);
	textos.current = lang;
	useEffect(() => {
		let avisada: string | null = null;
		return useActualizacion.subscribe((s, antes) => {
			const { t, mensaje } = textos.current;
			const info = s.info;
			if (!info) return;
			if (s.paso === "bajando" && (antes.paso !== "bajando" || s.progreso !== antes.progreso)) {
				toast.loading(t("Descargando Proxy Builder {version}…", { version: info.version }), {
					id: ID_AVISO,
					description: s.progreso !== null ? `${s.progreso} %` : undefined,
				});
			} else if (s.paso === "lista" && antes.paso === "bajando" && avisada !== info.version) {
				avisada = info.version;
				avisar(info, textos.current);
			} else if (s.paso === "disponible" && antes.paso === "bajando") {
				toast.error(t("No se ha podido descargar la actualización."), {
					id: ID_AVISO,
					description: s.error ? mensaje(s.error) : undefined,
				});
			}
			if (s.alSalirDeLaMesa && !antes.alSalirDeLaMesa) {
				toast.info(t("Se instalará al salir de la mesa."), { id: ID_AVISO });
			}
		});
	}, []);
	// Pedida la instalación con la mesa abierta: al salir de ella, se hace.
	useEffect(
		() =>
			useMesa.subscribe((s, antes) => {
				const a = useActualizacion.getState();
				if (antes.activa && !s.activa && a.alSalirDeLaMesa) {
					a.instalar().catch((e) => toast.error(textos.current.mensaje(e)));
				}
			}),
		[],
	);
}

const ID_AVISO = "actualizacion";

function avisar(info: ActualizacionInfo, { t, mensaje }: ReturnType<typeof useLang>) {
	toast.success(t("Proxy Builder {version} está lista", { version: info.version }), {
		id: ID_AVISO,
		duration: Number.POSITIVE_INFINITY,
		description: info.pidePermiso
			? t("Se instala al reiniciar; el sistema pedirá tu contraseña.")
			: t("Se instala al reiniciar la app."),
		action: {
			label: t("Reiniciar"),
			onClick: () => {
				useActualizacion
					.getState()
					.instalar()
					.catch((e) => toast.error(mensaje(e)));
			},
		},
	});
}

/** En la barra superior, mientras haya una versión bajada sin instalar. */
export function BotonActualizar() {
	const { t, mensaje } = useLang();
	const paso = useActualizacion((s) => s.paso);
	const info = useActualizacion((s) => s.info);
	const esperando = useActualizacion((s) => s.alSalirDeLaMesa);
	if ((paso !== "lista" && paso !== "instalando") || !info) return null;
	const instalar = () => {
		toast.dismiss(ID_AVISO);
		useActualizacion
			.getState()
			.instalar()
			.catch((e) => toast.error(mensaje(e)));
	};
	return (
		<WithTooltip
			content={
				esperando
					? t("Se instalará al salir de la mesa.")
					: info.pidePermiso
						? t("Reinicia la app con la versión nueva. El sistema pedirá tu contraseña.")
						: t("Reinicia la app con la versión nueva.")
			}
		>
			<Button
				size="sm"
				variant="outline"
				onClick={instalar}
				disabled={paso === "instalando" || esperando}
			>
				{paso === "instalando" ? (
					<Loader2 className="size-4 animate-spin" />
				) : (
					<ArrowUpCircle className="size-4 text-primary" />
				)}
				{t("Actualizar a {version}", { version: info.version })}
			</Button>
		</WithTooltip>
	);
}

/**
 * El puntito sobre el botón de Ajustes: ámbar y latiendo mientras se baja una
 * versión nueva; del color de la app cuando está lista para instalarse.
 */
export function PuntoActualizacion() {
	const paso = useActualizacion((s) => s.paso);
	if (paso !== "bajando" && paso !== "lista" && paso !== "instalando") return null;
	return (
		<span
			aria-hidden
			className={cn(
				"absolute top-1 right-1 size-2 rounded-full ring-2 ring-background",
				paso === "bajando" ? "animate-pulse bg-amber-500" : "bg-primary",
			)}
		/>
	);
}

/** Lo que dice el botón de Ajustes mientras hay una actualización en marcha. */
export function useAvisoAjustes(): string | null {
	const { t } = useLang();
	const paso = useActualizacion((s) => s.paso);
	const info = useActualizacion((s) => s.info);
	if (!info) return null;
	if (paso === "bajando") return t("Descargando la {version}…", { version: info.version });
	if (paso === "lista" || paso === "instalando")
		return t("La {version} está lista: reinicia desde Ajustes.", { version: info.version });
	return null;
}
