import { Plug, X } from "lucide-react";
import { useEffect, useState } from "react";
import type { VueltaRed } from "@/bindings/VueltaRed";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { localeDe } from "@/lib/idiomas";
import { call } from "@/lib/ipc";
import { useRed } from "@/stores/red-store";
import { useTrabajo } from "@/stores/trabajo-store";
import { volverARed } from "./conexion-red";

/**
 * **Volver a una partida en red** de la que este invitado se cayó (se cortó,
 * se cerró la app…): Rust recuerda el código (`red/vuelta.rs`) y el
 * anfitrión le devuelve su asiento. Sale en la entrada de «Jugar» mientras
 * no se esté en otra partida en red.
 */
export function VolverARed() {
	const { t, lang } = useLang();
	const [vuelta, setVuelta] = useState<VueltaRed | null>(null);
	const enRed = useRed((s) => s.papel !== null);
	const trabajo = useTrabajo((s) => s.actual !== null);
	useEffect(() => {
		if (enRed) return;
		call("red_vuelta")
			.then(setVuelta)
			.catch(() => setVuelta(null));
	}, [enRed]);
	if (!vuelta || enRed) return null;
	const cuando = new Intl.DateTimeFormat(localeDe(lang), {
		dateStyle: "short",
		timeStyle: "short",
	}).format(new Date(vuelta.cuandoMs));
	const olvidar = () => {
		setVuelta(null);
		void call("red_vuelta_olvidar").catch(() => {});
	};
	const volver = async () => {
		if (!(await volverARed())) {
			// Ya no estaba (Rust la olvida si el anfitrión la rechaza).
			call("red_vuelta")
				.then(setVuelta)
				.catch(() => setVuelta(null));
		}
	};
	return (
		<section className="flex flex-col gap-2.5 rounded-lg border border-primary/40 bg-primary/5 p-2.5">
			<h3 className="text-xs font-semibold">{t("Partida en red a medias")}</h3>
			<p className="text-xs text-muted-foreground">
				{t("Te desconectaste de una partida en red ({cuando}).", { cuando })}
			</p>
			<div className="flex gap-2.5">
				<Button size="sm" className="flex-1" disabled={trabajo} onClick={() => void volver()}>
					<Plug className="size-3.5" />
					{t("Volver a la partida")}
				</Button>
				<Button size="sm" variant="outline" onClick={olvidar}>
					<X className="size-3.5" />
					{t("Olvidar")}
				</Button>
			</div>
		</section>
	);
}
