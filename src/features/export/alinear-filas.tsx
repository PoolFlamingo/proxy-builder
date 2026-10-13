import { Crosshair, RotateCcw } from "lucide-react";
import { useState } from "react";
import type { Offset } from "@/bindings/Offset";
import type { RowOffsets } from "@/bindings/RowOffsets";
import { CampoNumero } from "@/components/campos";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogCuerpo,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { useDeck } from "@/stores/deck-store";
import { PresetsOffsets } from "./presets";

const CERO: Offset = { xMm: 0, yMm: 0 };

function fila(lista: Offset[], i: number): Offset {
	return lista[i] ?? CERO;
}

function conFila(lista: Offset[], i: number, o: Offset, filas: number): Offset[] {
	const out = Array.from({ length: Math.max(filas, lista.length) }, (_, k) => fila(lista, k));
	out[i] = o;
	return out;
}

/**
 * «Alinear filas»: cuánto se mueve cada fila al imprimir, en el frente y en
 * el reverso. 🛑 Los del reverso se dan en el marco del FRENTE (lo que se ve
 * mirando la hoja al trasluz desde delante); Rust los aplica antes del espejo.
 */
export function AlinearFilas({ filas }: { filas: number }) {
	const { t } = useLang();
	const offsets = useDeck((s) => s.deck?.settings.rowOffsets);
	const setAjustes = useDeck((s) => s.setAjustes);
	const [medir, setMedir] = useState(false);
	if (!offsets) return null;

	const poner = (cara: keyof RowOffsets, i: number, o: Offset) =>
		setAjustes((s) => ({
			...s,
			rowOffsets: { ...s.rowOffsets, [cara]: conFila(s.rowOffsets[cara], i, o, filas) },
		}));

	const tabla = (cara: keyof RowOffsets, titulo: string) => (
		<div className="flex flex-col gap-1">
			<div className="grid grid-cols-[2rem_1fr_1fr] items-center gap-1.5 text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
				<span>{titulo}</span>
				<span className="text-center">X</span>
				<span className="text-center">Y</span>
			</div>
			{Array.from({ length: filas }, (_, i) => {
				const o = fila(offsets[cara], i);
				return (
					<div key={i} className="grid grid-cols-[2rem_1fr_1fr] items-center gap-1.5">
						<span className="font-mono text-[11px] text-muted-foreground">R{i + 1}</span>
						<CampoNumero
							valor={o.xMm}
							onChange={(v) => poner(cara, i, { ...o, xMm: v ?? 0 })}
							min={-20}
							max={20}
							paso={0.1}
							sufijo="mm"
							ancho="w-full"
							ariaLabel={`${titulo} R${i + 1} X`}
						/>
						<CampoNumero
							valor={o.yMm}
							onChange={(v) => poner(cara, i, { ...o, yMm: v ?? 0 })}
							min={-20}
							max={20}
							paso={0.1}
							sufijo="mm"
							ancho="w-full"
							ariaLabel={`${titulo} R${i + 1} Y`}
						/>
					</div>
				);
			})}
		</div>
	);

	return (
		<div className="flex flex-col gap-2.5">
			{tabla("front", t("Frente"))}
			<PresetsOffsets kind="frontOffsets" />
			{tabla("back", t("Reverso"))}
			<PresetsOffsets kind="backOffsets" />
			<div className="grid grid-cols-2 gap-1.5">
				<Button size="sm" variant="outline" onClick={() => setMedir(true)}>
					<Crosshair />
					{t("Corregir con lo medido")}
				</Button>
				<Button
					size="sm"
					variant="ghost"
					onClick={() => setAjustes((s) => ({ ...s, rowOffsets: { front: [], back: [] } }))}
				>
					<RotateCcw />
					{t("Restablecer")}
				</Button>
			</div>
			<CorregirConLoMedido abierto={medir} onCerrar={() => setMedir(false)} filas={filas} />
		</div>
	);
}

/**
 * Se teclea lo leído en la hoja de calibración (cuánto se desvía la cruz
 * del reverso, hacia la derecha y hacia abajo en positivo) y se suma su
 * opuesto al desplazamiento del reverso de cada fila.
 */
function CorregirConLoMedido({
	abierto,
	onCerrar,
	filas,
}: {
	abierto: boolean;
	onCerrar: () => void;
	filas: number;
}) {
	const { t } = useLang();
	const [medidas, setMedidas] = useState<Offset[]>([]);
	const aplicar = () => {
		useDeck.getState().setAjustes((s) => {
			const back = Array.from({ length: Math.max(filas, s.rowOffsets.back.length) }, (_, i) => {
				const a = fila(s.rowOffsets.back, i);
				const m = fila(medidas, i);
				return {
					xMm: Number((a.xMm - m.xMm).toFixed(2)),
					yMm: Number((a.yMm - m.yMm).toFixed(2)),
				};
			});
			return { ...s, rowOffsets: { ...s.rowOffsets, back } };
		});
		setMedidas([]);
		onCerrar();
	};
	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && onCerrar()}>
			<DialogContent className="sm:max-w-sm">
				<DialogHeader>
					<DialogTitle>{t("Corregir con lo medido")}</DialogTitle>
					<DialogDescription>
						{t(
							"Mirando la hoja de calibración al trasluz desde el frente, ¿cuánto se desvía la cruz roja de cada fila? Derecha y abajo, en positivo.",
						)}
					</DialogDescription>
				</DialogHeader>
				<DialogCuerpo>
					<div className="flex flex-col gap-1.5">
						{Array.from({ length: filas }, (_, i) => {
							const m = fila(medidas, i);
							return (
								<div key={i} className="grid grid-cols-[2rem_1fr_1fr] items-center gap-1.5">
									<span className="font-mono text-[11px] text-muted-foreground">R{i + 1}</span>
									<CampoNumero
										valor={m.xMm}
										onChange={(v) => setMedidas((l) => conFila(l, i, { ...m, xMm: v ?? 0 }, filas))}
										min={-20}
										max={20}
										sufijo="mm"
										ancho="w-full"
										ariaLabel={`R${i + 1} X`}
									/>
									<CampoNumero
										valor={m.yMm}
										onChange={(v) => setMedidas((l) => conFila(l, i, { ...m, yMm: v ?? 0 }, filas))}
										min={-20}
										max={20}
										sufijo="mm"
										ancho="w-full"
										ariaLabel={`R${i + 1} Y`}
									/>
								</div>
							);
						})}
					</div>
				</DialogCuerpo>
				<DialogFooter>
					<Button size="sm" variant="outline" onClick={onCerrar}>
						{t("Cancelar")}
					</Button>
					<Button size="sm" onClick={aplicar}>
						{t("Corregir el reverso")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
