import { useState } from "react";
import { CampoNumero } from "@/components/campos";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { moverA } from "./acciones-carta";

/** «Mover a posición…»: el número de casilla (1 = la primera del deck). */
export function MoverADialog() {
	const { t } = useLang();
	const dialogo = useUi((s) => s.dialogo);
	const ids = dialogo?.tipo === "moverA" ? dialogo.ids : null;
	const total = useDeck((s) => s.deck?.cards.length ?? 0);
	const [pos, setPos] = useState(1);
	const cerrar = () => useUi.getState().abrir(null);
	return (
		<Dialog open={!!ids} onOpenChange={(o) => !o && cerrar()}>
			<DialogContent className="sm:max-w-xs" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Mover a posición")}</DialogTitle>
					<DialogDescription>
						{t("De 1 a {n}. Cada página tiene sus casillas seguidas.", { n: total })}
					</DialogDescription>
				</DialogHeader>
				<CampoNumero
					valor={pos}
					onChange={(v) => setPos(Math.max(1, Math.min(total, Math.round(v ?? 1))))}
					min={1}
					max={total}
					paso={1}
					ancho="w-full"
					ariaLabel={t("Posición")}
				/>
				<DialogFooter>
					<Button size="sm" variant="outline" onClick={cerrar}>
						{t("Cancelar")}
					</Button>
					<Button
						size="sm"
						onClick={() => {
							if (ids) moverA(ids, pos);
							cerrar();
						}}
					>
						{t("Mover")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
