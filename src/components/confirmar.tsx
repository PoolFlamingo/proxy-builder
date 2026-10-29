import { type ReactNode, useState } from "react";
import { Icono } from "@/components/icono";
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
 *
 * Con `accionAlternativa` hay una tercera salida, la buena («Guardar y
 * continuar»): se pone a la derecha y es la que tiene el foco, y la acción
 * `accion` (la que tira lo hecho) se queda en medio. `onClick` devuelve si
 * salió bien: con `false` el diálogo sigue abierto (quien falló ya avisó) y
 * mientras trabaja no se puede pulsar nada ni cerrar.
 */
export function Confirmar({
	abierto,
	onCerrar,
	titulo,
	descripcion,
	accion,
	destructiva,
	onConfirmar,
	accionAlternativa,
	children,
}: {
	abierto: boolean;
	onCerrar: () => void;
	titulo: string;
	descripcion?: ReactNode;
	accion: string;
	destructiva?: boolean;
	onConfirmar: () => void;
	accionAlternativa?: { texto: string; onClick: () => Promise<boolean> | boolean };
	children?: ReactNode;
}) {
	const { t } = useLang();
	const [ocupado, setOcupado] = useState(false);
	const alternativa = async () => {
		if (!accionAlternativa || ocupado) return;
		setOcupado(true);
		try {
			if (await accionAlternativa.onClick()) onCerrar();
		} finally {
			setOcupado(false);
		}
	};
	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && !ocupado && onCerrar()}>
			<DialogContent className="sm:max-w-md" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{titulo}</DialogTitle>
					{descripcion && <DialogDescription>{descripcion}</DialogDescription>}
				</DialogHeader>
				{children}
				<DialogFooter>
					<Button variant="outline" size="sm" onClick={onCerrar} disabled={ocupado}>
						{t("Cancelar")}
					</Button>
					<Button
						size="sm"
						variant={destructiva ? "destructive" : accionAlternativa ? "outline" : "default"}
						autoFocus={!accionAlternativa}
						disabled={ocupado}
						onClick={() => {
							onCerrar();
							onConfirmar();
						}}
					>
						{accion}
					</Button>
					{accionAlternativa && (
						<Button size="sm" autoFocus disabled={ocupado} onClick={() => void alternativa()}>
							{ocupado && <Icono icon="lucide:loader-2" className="animate-spin" />}
							{accionAlternativa.texto}
						</Button>
					)}
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
