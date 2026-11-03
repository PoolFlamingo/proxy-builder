import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { PackFaltante } from "@/bindings/PackFaltante";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useMarcosOriginales } from "./marcos-originales";
import { usePacksDescargables } from "./packs-descargables";

/**
 * **Marcos que faltan.** Un deck que llega de otro equipo (o una colección)
 * puede pedir marcos que aquí no se han bajado (los «Original», Balatro):
 * sin ellos Rust pinta esas cartas con los de la casa. Este aviso lo dice y
 * los baja con un botón; al terminar, quien lista los packs los vuelve a pedir
 * (`version` de los dos stores) y las cartas se pintan con su marco.
 */
export function AvisoPacksFaltan({
	deckId,
	className,
}: {
	deckId: string | null | undefined;
	className?: string;
}) {
	const { t, mensaje } = useLang();
	const [faltan, setFaltan] = useState<PackFaltante[]>([]);
	const [bajando, setBajando] = useState(false);
	const versionOriginales = useMarcosOriginales((s) => s.version);
	const versionDescargables = usePacksDescargables((s) => s.version);

	useEffect(() => {
		void versionOriginales;
		void versionDescargables;
		if (!deckId) {
			setFaltan([]);
			return;
		}
		let vivo = true;
		call("packs_missing", { deckId })
			.then((f) => vivo && setFaltan(f))
			.catch(() => vivo && setFaltan([]));
		return () => {
			vivo = false;
		};
	}, [deckId, versionOriginales, versionDescargables]);

	if (faltan.length === 0) return null;

	const descargar = async () => {
		setBajando(true);
		try {
			for (const p of faltan) {
				if (p.original) await useMarcosOriginales.getState().descargar();
				else await usePacksDescargables.getState().descargar(p.id);
			}
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setBajando(false);
		}
	};

	return (
		<div
			role="status"
			className={cn(
				"flex items-center gap-2.5 rounded-[14px] border border-amber-500/40 bg-amber-500/10 p-2.5 text-xs",
				className,
			)}
		>
			<Icono icon="lucide:triangle-alert" className="size-4 shrink-0 text-amber-500" />
			<span className="min-w-0 flex-1">
				{t("Faltan los marcos {packs}: estas cartas se pintan con los de la casa.", {
					packs: faltan.map((p) => p.nombre).join(", "),
				})}
			</span>
			<Button size="sm" variant="outline" disabled={bajando} onClick={() => void descargar()}>
				<Icono
					icon={bajando ? "lucide:loader-2" : "lucide:download"}
					className={bajando ? "animate-spin" : undefined}
				/>
				{t("Descargar")}
			</Button>
		</div>
	);
}
