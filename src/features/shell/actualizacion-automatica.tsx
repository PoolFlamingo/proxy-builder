import { ArrowUpCircle, Loader2 } from "lucide-react";
import { useEffect, useRef } from "react";
import { toast } from "sonner";
import type { ActualizacionInfo } from "@/bindings/ActualizacionInfo";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { useActualizacion } from "@/stores/actualizacion-store";
import { useUi } from "@/stores/ui-store";

/** La primera búsqueda espera a que la app haya arrancado del todo. */
const PRIMERA_MS = 8_000;
const CADA_MS = 6 * 60 * 60 * 1000;

/**
 * Si está encendido en Ajustes (lo está de fábrica): busca una versión nueva
 * al arrancar y cada 6 horas, la baja sin molestar y avisa para reiniciar.
 * Sin red o sin release no dice nada.
 */
export function useActualizacionAutomatica() {
	const lang = useLang();
	const textos = useRef(lang);
	textos.current = lang;
	const activa = useUi((s) => s.actualizarSola);
	const avisada = useRef<string | null>(null);

	useEffect(() => {
		if (!activa) return;
		let viva = true;
		const ronda = async () => {
			const a = useActualizacion.getState();
			if (!(await a.iniciar()) || !viva) return;
			const info = await a.buscar();
			if (!info || !viva) return;
			if (!(await useActualizacion.getState().bajar()) || !viva) return;
			if (avisada.current === info.version) return;
			avisada.current = info.version;
			avisar(info, textos.current);
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

function avisar(info: ActualizacionInfo, { t, mensaje }: ReturnType<typeof useLang>) {
	toast(t("Proxy Builder {version} está lista", { version: info.version }), {
		id: "actualizacion",
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
	if ((paso !== "lista" && paso !== "instalando") || !info) return null;
	const instalar = () => {
		toast.dismiss("actualizacion");
		useActualizacion
			.getState()
			.instalar()
			.catch((e) => toast.error(mensaje(e)));
	};
	return (
		<WithTooltip
			content={
				info.pidePermiso
					? t("Reinicia la app con la versión nueva. El sistema pedirá tu contraseña.")
					: t("Reinicia la app con la versión nueva.")
			}
		>
			<Button size="sm" variant="outline" onClick={instalar} disabled={paso === "instalando"}>
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
