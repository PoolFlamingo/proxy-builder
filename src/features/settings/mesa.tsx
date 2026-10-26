import { Swords } from "lucide-react";
import { Fila, FilaDeslizador, FilaInterruptor, Seccion, Selector } from "@/components/campos";
import { useLang } from "@/components/language-provider";
import { MODO_RENDIMIENTO } from "@/features/mesa/fps";
import { type NivelDados, useUi } from "@/stores/ui-store";
import { AjustesDados } from "./aspecto-dados";

/**
 * **Ajustes > Mesa** (`capturas/pablo/19.jpg`): jugar, tablero, dados y
 * monedas (aspecto y sonido, `aspecto-dados.tsx`) y rendimiento. El resto de
 * sonidos de la mesa llega con la Fase H.
 */
export function AjustesMesa() {
	const { t } = useLang();
	const m = useUi((s) => s.mesa);
	const set = useUi((s) => s.setMesa);
	const modoRendimiento = () => set(MODO_RENDIMIENTO);
	return (
		<Seccion
			id="ajustes-mesa"
			titulo={t("Mesa de juego")}
			icono={<Swords className="size-3.5" />}
			abiertaAlPrincipio={false}
		>
			<h4 className="text-[11px] font-semibold text-muted-foreground uppercase">{t("Jugar")}</h4>
			<FilaInterruptor
				etiqueta={t("Ofrecer el bono de auras y equipos al adjuntarlos")}
				valor={m.ofrecerBono}
				onChange={(v) => set({ ofrecerBono: v })}
			/>
			<FilaInterruptor
				etiqueta={t("Ver la vida del fantasma en solitario")}
				valor={m.fantasma}
				onChange={(v) => set({ fantasma: v })}
			/>
			<FilaInterruptor
				etiqueta={t("Mantener pulsado abre el menú de la carta")}
				valor={m.clicSostenido}
				onChange={(v) => set({ clicSostenido: v })}
			/>
			<h4 className="text-[11px] font-semibold text-muted-foreground uppercase">{t("Tablero")}</h4>
			<FilaDeslizador
				etiqueta={t("Tamaño de las cartas del campo")}
				valor={m.tamanoCartas}
				onChange={(v) => set({ tamanoCartas: v })}
				min={25}
				max={200}
				paso={5}
				sufijo=" %"
			/>
			<FilaDeslizador
				etiqueta={t("Tamaño de la vista grande")}
				valor={m.vistaGrande}
				onChange={(v) => set({ vistaGrande: v })}
				min={200}
				max={500}
				paso={10}
				sufijo=" px"
			/>
			<FilaInterruptor
				etiqueta={t("Panel de estado junto a la vista grande")}
				valor={m.panelEstado}
				onChange={(v) => set({ panelEstado: v })}
			/>
			<AjustesDados />
			<h4 className="text-[11px] font-semibold text-muted-foreground uppercase">
				{t("Rendimiento")}
			</h4>
			<Fila etiqueta={t("Calidad de las imágenes del campo")}>
				<Selector<string>
					valor={String(m.calidad)}
					onChange={(v) => set({ calidad: Number(v) as 146 | 244 | 488 })}
					opciones={[
						{ valor: "146", etiqueta: t("Baja") },
						{ valor: "244", etiqueta: t("Media") },
						{ valor: "488", etiqueta: t("Alta") },
					]}
				/>
			</Fila>
			<Fila etiqueta={t("Dados y monedas")}>
				<Selector<NivelDados>
					valor={m.dados}
					onChange={(v) => set({ dados: v })}
					opciones={[
						{ valor: "auto", etiqueta: t("Automático") },
						{ valor: "3d", etiqueta: t("3D con física") },
						{ valor: "css", etiqueta: t("3D ligero") },
						{ valor: "2d", etiqueta: t("2D") },
					]}
				/>
			</Fila>
			<FilaInterruptor
				etiqueta={t("Reducir animaciones")}
				valor={m.reducirAnimaciones}
				onChange={(v) => set({ reducirAnimaciones: v })}
			/>
			<FilaInterruptor
				etiqueta={t("Sombras")}
				valor={m.sombras}
				onChange={(v) => set({ sombras: v })}
			/>
			<FilaInterruptor
				etiqueta={t("Contador de fotogramas")}
				valor={m.fps}
				onChange={(v) => set({ fps: v })}
			/>
			<button
				type="button"
				onClick={modoRendimiento}
				className="self-start text-xs text-primary underline-offset-2 hover:underline"
			>
				{t("Modo rendimiento (todo al mínimo)")}
			</button>
		</Seccion>
	);
}
