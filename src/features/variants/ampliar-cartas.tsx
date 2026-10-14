import type { CardSummary } from "@/bindings/CardSummary";
import { useLang } from "@/components/language-provider";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { EDICION_NEUTRA, parametros, urlBusqueda, urlMiniatura } from "@/lib/image-url";
import { useDeck } from "@/stores/deck-store";

/**
 * **Las dos cartas en grande**, una al lado de la otra: la imagen propia y la
 * de Scryfall propuesta, para comparar antes de vincular. Se abre al pulsar
 * cualquiera de las dos miniaturas de una fila.
 */
export function AmpliarCartas({
	blob,
	original,
	onCerrar,
}: {
	/** La imagen propia (`null` = cerrado). */
	blob: string | null;
	original: CardSummary | null;
	onCerrar: () => void;
}) {
	const { t } = useLang();
	const ajustes = useDeck((s) => s.deck?.settings);
	return (
		<Dialog open={blob !== null} onOpenChange={(o) => !o && onCerrar()}>
			{blob !== null && (
				<DialogContent
					className="flex max-h-[94vh] flex-col sm:max-w-5xl"
					cambiosSinGuardar={false}
				>
					<DialogHeader>
						<DialogTitle className="text-sm">{t("Comparar")}</DialogTitle>
						<DialogDescription className="text-xs">
							{original
								? `${original.printedName ?? original.name} · ${original.set.toUpperCase()} ${original.collectorNumber}`
								: t("Sin carta de Scryfall elegida.")}
						</DialogDescription>
					</DialogHeader>
					<div className="grid min-h-0 flex-1 grid-cols-2 gap-4">
						<Lado titulo={t("Tu carta")}>
							{ajustes && (
								<img
									src={urlMiniatura(blob, parametros(ajustes, EDICION_NEUTRA, false, 0), 745)}
									alt=""
									className="max-h-full max-w-full rounded-[4.5%] object-contain"
								/>
							)}
						</Lado>
						<Lado titulo={t("Scryfall")}>
							{original?.thumb ? (
								<img
									src={urlBusqueda(original.thumb)}
									alt=""
									className="max-h-full max-w-full rounded-[4.5%] object-contain"
								/>
							) : (
								<span className="text-xs text-muted-foreground">—</span>
							)}
						</Lado>
					</div>
				</DialogContent>
			)}
		</Dialog>
	);
}

function Lado({ titulo, children }: { titulo: string; children: React.ReactNode }) {
	return (
		<div className="flex min-h-0 flex-col items-center gap-1.5">
			<span className="text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
				{titulo}
			</span>
			<div className="flex h-[min(76vh,880px)] w-full items-center justify-center">{children}</div>
		</div>
	);
}
