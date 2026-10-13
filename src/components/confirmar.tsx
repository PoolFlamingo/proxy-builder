import type { ReactNode } from "react";
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

/**
 * Una confirmación corta («¿Reemplazar las cartas?»). Quien la abre guarda el
 * estado; `onConfirmar` se llama al pulsar el botón principal y el diálogo se
 * cierra solo.
 */
export function Confirmar({
	abierto,
	onCerrar,
	titulo,
	descripcion,
	accion,
	destructiva,
	onConfirmar,
	children,
}: {
	abierto: boolean;
	onCerrar: () => void;
	titulo: string;
	descripcion?: ReactNode;
	accion: string;
	destructiva?: boolean;
	onConfirmar: () => void;
	children?: ReactNode;
}) {
	const { t } = useLang();
	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && onCerrar()}>
			<DialogContent className="sm:max-w-md" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{titulo}</DialogTitle>
					{descripcion && <DialogDescription>{descripcion}</DialogDescription>}
				</DialogHeader>
				{children}
				<DialogFooter>
					<Button variant="outline" size="sm" onClick={onCerrar}>
						{t("Cancelar")}
					</Button>
					<Button
						size="sm"
						variant={destructiva ? "destructive" : "default"}
						autoFocus
						onClick={() => {
							onCerrar();
							onConfirmar();
						}}
					>
						{accion}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
