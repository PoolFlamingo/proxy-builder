import type { DeckStats } from "@/bindings/DeckStats";
import { useLang } from "@/components/language-provider";
import { Bloque, Lista, ListaCartas } from "./piezas";
import { FUNCIONES } from "./textos";

/** Qué hace cada carta: rampa, robo, eliminación… */
export function Funciones({ s }: { s: DeckStats }) {
	const { t } = useLang();
	const con = s.roles.filter((r) => r.count > 0);
	const sin = s.roles.filter((r) => r.count === 0);
	return (
		<div className="flex flex-col gap-2.5">
			<Bloque titulo={t("Funciones")}>
				<p className="text-[11px] text-muted-foreground">
					{t(
						"Se deduce del texto de reglas de cada carta: es orientativo. Una carta puede tener varias funciones. El botón de la derecha selecciona esas cartas en el deck.",
					)}
				</p>
				<Lista
					ancho="9rem"
					filas={con.map((r) => ({
						k: r.key,
						n: r.count,
						txt: t(FUNCIONES[r.key]?.nombre ?? r.key),
					}))}
					vacio={t("No se ha detectado ninguna.")}
				/>
			</Bloque>
			<Bloque titulo={t("Cartas de cada función")}>
				<div className="flex flex-col gap-1">
					{con.map((r) => (
						<ListaCartas
							key={r.key}
							titulo={t(FUNCIONES[r.key]?.nombre ?? r.key)}
							ayuda={t(FUNCIONES[r.key]?.ayuda ?? "")}
							cartas={r.cards}
						/>
					))}
				</div>
				{sin.length > 0 && (
					<p className="text-[11px] text-muted-foreground">
						{t("Sin cartas: {f}", {
							f: sin.map((r) => t(FUNCIONES[r.key]?.nombre ?? r.key)).join(", "),
						})}
					</p>
				)}
			</Bloque>
		</div>
	);
}
