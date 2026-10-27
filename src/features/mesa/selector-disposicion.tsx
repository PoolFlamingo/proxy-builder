import { LayoutGrid, PanelBottom, Rows2, Square, Users } from "lucide-react";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuLabel,
	DropdownMenuRadioGroup,
	DropdownMenuRadioItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { useMesa } from "@/stores/mesa-store";
import { useUi } from "@/stores/ui-store";
import {
	type ConfigDisposicion,
	disposicionDe,
	type LadoTira,
	type ModoDisposicion,
	modosPara,
} from "./disposicion";

const ICONOS: Record<ModoDisposicion, typeof Square> = {
	unico: Square,
	dividido: Rows2,
	foco: PanelBottom,
	rejilla: LayoutGrid,
	equipos: Users,
};

/** La disposición que vale ahora y cómo cambiarla (se recuerda por número de jugadores). */
export function useDisposicion(): {
	n: number;
	conEquipos: boolean;
	config: ConfigDisposicion;
	cambiar: (c: Partial<ConfigDisposicion>) => void;
} {
	const n = useMesa((s) => s.asientos.length);
	const guardadas = useUi((s) => s.mesa.disposicion);
	const conEquipos = useMesa((s) => (s.general?.equipos.length ?? 0) > 0);
	const config = disposicionDe(guardadas, n, conEquipos);
	const cambiar = (c: Partial<ConfigDisposicion>) =>
		useUi.getState().setMesa({
			disposicion: { ...useUi.getState().mesa.disposicion, [n]: { ...config, ...c } },
		});
	return { n, conEquipos, config, cambiar };
}

/**
 * **Cómo se reparten los tableros** de la mesa (un menú en la barra): un
 * tablero, dos lado a lado, uno grande con miniaturas, todos iguales o por
 * equipos. Con un solo jugador no aparece.
 */
export function SelectorDisposicion({ className }: { className?: string }) {
	const { t } = useLang();
	const { n, conEquipos, config, cambiar } = useDisposicion();
	if (n < 2) return null;
	const etiqueta: Record<ModoDisposicion, string> = {
		unico: t("Un solo tablero"),
		dividido: t("Dos tableros"),
		foco: t("Uno grande y los demás en miniatura"),
		rejilla: t("Todos iguales"),
		equipos: t("Por equipos"),
	};
	const Icono = ICONOS[config.modo];
	return (
		<DropdownMenu>
			<WithTooltip content={t("Disposición de los tableros")}>
				<DropdownMenuTrigger asChild>
					<Button
						size="icon-sm"
						variant="ghost"
						aria-label={t("Disposición de los tableros")}
						className={className}
					>
						<Icono className="size-4" />
					</Button>
				</DropdownMenuTrigger>
			</WithTooltip>
			<DropdownMenuContent align="center" className="z-[9000] w-72">
				<DropdownMenuLabel>{t("Disposición de los tableros")}</DropdownMenuLabel>
				<DropdownMenuRadioGroup
					value={config.modo}
					onValueChange={(v) => cambiar({ modo: v as ModoDisposicion })}
				>
					{modosPara(n, conEquipos).map((m) => (
						<DropdownMenuRadioItem key={m} value={m}>
							{etiqueta[m]}
						</DropdownMenuRadioItem>
					))}
				</DropdownMenuRadioGroup>
				{config.modo === "foco" && (
					<>
						<DropdownMenuSeparator />
						<DropdownMenuLabel>{t("Tira de miniaturas")}</DropdownMenuLabel>
						<DropdownMenuRadioGroup
							value={config.tira}
							onValueChange={(v) => cambiar({ tira: v as LadoTira })}
						>
							<DropdownMenuRadioItem value="arriba">{t("Arriba")}</DropdownMenuRadioItem>
							<DropdownMenuRadioItem value="derecha">{t("A la derecha")}</DropdownMenuRadioItem>
							<DropdownMenuRadioItem value="abajo">{t("Abajo")}</DropdownMenuRadioItem>
						</DropdownMenuRadioGroup>
					</>
				)}
				{config.modo === "dividido" && (
					<>
						<DropdownMenuSeparator />
						<DropdownMenuRadioGroup
							value={config.vertical ? "lado" : "encima"}
							onValueChange={(v) => cambiar({ vertical: v === "lado" })}
						>
							<DropdownMenuRadioItem value="lado">
								{t("Uno al lado del otro")}
							</DropdownMenuRadioItem>
							<DropdownMenuRadioItem value="encima">
								{t("Uno encima del otro")}
							</DropdownMenuRadioItem>
						</DropdownMenuRadioGroup>
					</>
				)}
			</DropdownMenuContent>
		</DropdownMenu>
	);
}
