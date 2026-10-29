import { useMemo, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import type { Nota } from "@/bindings/Nota";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { cn } from "@/lib/utils";
import { useMesa } from "@/stores/mesa-store";
import { useRepeticion } from "@/stores/repeticion-store";
import { hacer } from "./acciones";
import { MAX_NOTA, repartirNotas } from "./cuentas-notas";

/**
 * **Las notas de la mesa**: lo que cada jugador se apunta para acordarse
 * (el maná que dejó sacrificar un tesoro, un efecto que dura hasta el final
 * del turno…). Son `Accion::PonerNota` / `EditarNota` / `QuitarNota` del
 * motor; una **privada** solo la lee quien la escribió (Rust ni se la manda
 * a los demás en la vista) y una **temporal** se borra sola al pasar el
 * turno. Cada uno toca solo las suyas.
 */
export function NotasPanel() {
	const { t } = useLang();
	const notas = useMesa((s) => s.general?.notas);
	const yo = useMesa((s) => s.yo);
	const asientos = useMesa(useShallow((s) => s.asientos));
	const jugadores = useMesa((s) => s.jugadores);
	const repitiendo = useRepeticion((s) => s.activa !== null);
	const [texto, setTexto] = useState("");
	const [temporal, setTemporal] = useState(false);
	const [privada, setPrivada] = useState(false);
	const puedeEscribir = yo !== null && !repitiendo;
	const { mias, ajenas } = useMemo(
		() => repartirNotas(notas ?? [], yo, asientos),
		[notas, yo, asientos],
	);

	const anadir = () => {
		const limpio = texto.trim();
		if (!limpio || !puedeEscribir) return;
		void hacer({ que: "ponerNota", texto: limpio, temporal, privada }).then(
			(ok) => ok && setTexto(""),
		);
	};

	return (
		<div className="flex min-h-0 flex-1 flex-col gap-2">
			<ScrollArea className="min-h-0 flex-1" viewportClassName="h-full">
				<div className="flex flex-col gap-1.5 p-1 text-xs">
					{mias.length === 0 && ajenas.length === 0 && (
						<p className="p-4 text-center text-muted-foreground">{t("Todavía no hay notas.")}</p>
					)}
					{mias.length > 0 && (
						<>
							<Cabecera>{t("Mis notas")}</Cabecera>
							{mias.map((n) => (
								<NotaPropia key={n.id} nota={n} puedeEditar={puedeEscribir} />
							))}
						</>
					)}
					{ajenas.map(({ autor, notas: delAutor }) => (
						<div key={autor} className="flex flex-col gap-1.5">
							<Cabecera>
								<span
									className="mr-1 inline-block size-2 rounded-full align-middle"
									style={{ background: jugadores[autor]?.color }}
								/>
								{jugadores[autor]?.nombre ?? "?"}
							</Cabecera>
							{delAutor.map((n) => (
								<div
									key={n.id}
									className="flex items-start gap-1.5 rounded-lg bg-muted/40 px-2 py-1"
								>
									<span className="min-w-0 flex-1 break-words whitespace-pre-wrap">{n.texto}</span>
									{n.temporal && (
										<Marca
											icono="game-icons:hourglass"
											ayuda={t("Se borra al terminar el turno")}
										/>
									)}
								</div>
							))}
						</div>
					))}
				</div>
			</ScrollArea>
			{puedeEscribir ? (
				<div className="flex flex-col gap-1.5">
					<div className="flex gap-1.5">
						<Input
							value={texto}
							maxLength={MAX_NOTA}
							placeholder={t("Apunta algo: el maná de un tesoro, un recordatorio…")}
							aria-label={t("Nueva nota")}
							onChange={(e) => setTexto(e.target.value)}
							onKeyDown={(e) => {
								if (e.key === "Enter") {
									e.preventDefault();
									anadir();
								} else if (e.key === "Escape") {
									e.currentTarget.blur();
								}
							}}
						/>
						<Button
							size="icon"
							onClick={anadir}
							disabled={!texto.trim()}
							aria-label={t("Añadir nota")}
						>
							<Icono icon="lucide:plus" className="size-4" />
						</Button>
					</div>
					<div className="flex flex-wrap gap-1">
						<Interruptor
							activo={temporal}
							onClick={() => setTemporal((x) => !x)}
							ayuda={
								temporal
									? t("Temporal: se borra al terminar el turno")
									: t("Fija: se queda hasta que la borres")
							}
							icono="game-icons:hourglass"
							etiqueta={temporal ? t("Temporal") : t("Fija")}
						/>
						<Interruptor
							activo={privada}
							onClick={() => setPrivada((x) => !x)}
							ayuda={privada ? t("Privada: solo la ves tú") : t("Pública: la ven todos")}
							icono={privada ? "lucide:lock" : "lucide:lock-open"}
							etiqueta={privada ? t("Privada") : t("Pública")}
						/>
					</div>
				</div>
			) : (
				<p className="px-1 text-center text-[11px] text-muted-foreground">
					{repitiendo
						? t("En la repetición no se escribe.")
						: t("Los espectadores no pueden escribir.")}
				</p>
			)}
		</div>
	);
}

function Cabecera({ children }: { children: React.ReactNode }) {
	return (
		<div className="px-1 pt-1 text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
			{children}
		</div>
	);
}

/** Un icono sin acción, con su explicación. */
function Marca({ icono, ayuda }: { icono: string; ayuda: string }) {
	return (
		<WithTooltip content={ayuda}>
			<span className="mt-0.5 shrink-0 text-muted-foreground">
				<Icono icon={icono} className="size-3.5" />
			</span>
		</WithTooltip>
	);
}

function Interruptor({
	activo,
	onClick,
	ayuda,
	icono,
	etiqueta,
}: {
	activo: boolean;
	onClick: () => void;
	ayuda: string;
	icono: string;
	etiqueta: string;
}) {
	return (
		<WithTooltip content={ayuda}>
			<Button
				size="xs"
				variant={activo ? "secondary" : "outline"}
				className="gap-1"
				aria-pressed={activo}
				onClick={onClick}
			>
				<Icono icon={icono} className="size-3" />
				{etiqueta}
			</Button>
		</WithTooltip>
	);
}

/** Una nota mía: se edita en su sitio, se cambia lo temporal y lo privado con un clic y se borra. */
function NotaPropia({ nota, puedeEditar }: { nota: Nota; puedeEditar: boolean }) {
	const { t } = useLang();
	const [editando, setEditando] = useState<string | null>(null);
	const cambiar = (cambios: Partial<Pick<Nota, "texto" | "temporal" | "privada">>) =>
		hacer({
			que: "editarNota",
			id: nota.id,
			texto: nota.texto,
			temporal: nota.temporal,
			privada: nota.privada,
			...cambios,
		});
	const guardar = () => {
		const limpio = (editando ?? "").trim();
		setEditando(null);
		if (limpio && limpio !== nota.texto) void cambiar({ texto: limpio });
	};
	return (
		<div className="flex items-start gap-1 rounded-lg bg-muted/40 px-2 py-1">
			{editando !== null ? (
				<Input
					autoFocus
					value={editando}
					maxLength={MAX_NOTA}
					aria-label={t("Editar nota")}
					className="h-7"
					onChange={(e) => setEditando(e.target.value)}
					onBlur={guardar}
					onKeyDown={(e) => {
						if (e.key === "Enter") {
							e.preventDefault();
							guardar();
						} else if (e.key === "Escape") {
							setEditando(null);
						}
					}}
				/>
			) : (
				<span className="min-w-0 flex-1 self-center break-words whitespace-pre-wrap">
					{nota.texto}
				</span>
			)}
			{puedeEditar && editando === null && (
				<span className="flex shrink-0 items-center">
					<WithTooltip
						content={
							nota.temporal
								? t("Temporal: se borra al terminar el turno")
								: t("Fija: se queda hasta que la borres")
						}
					>
						<Button
							size="icon-sm"
							variant="ghost"
							aria-pressed={nota.temporal}
							aria-label={t("Temporal")}
							onClick={() => void cambiar({ temporal: !nota.temporal })}
						>
							<Icono
								icon="game-icons:hourglass"
								className={cn("size-3.5", !nota.temporal && "opacity-30")}
							/>
						</Button>
					</WithTooltip>
					<WithTooltip
						content={nota.privada ? t("Privada: solo la ves tú") : t("Pública: la ven todos")}
					>
						<Button
							size="icon-sm"
							variant="ghost"
							aria-pressed={nota.privada}
							aria-label={t("Privada")}
							onClick={() => void cambiar({ privada: !nota.privada })}
						>
							{nota.privada ? (
								<Icono icon="lucide:lock" className="size-3.5" />
							) : (
								<Icono icon="lucide:lock-open" className="size-3.5 opacity-30" />
							)}
						</Button>
					</WithTooltip>
					<WithTooltip content={t("Editar nota")}>
						<Button
							size="icon-sm"
							variant="ghost"
							aria-label={t("Editar nota")}
							onClick={() => setEditando(nota.texto)}
						>
							<Icono icon="lucide:pencil" className="size-3.5" />
						</Button>
					</WithTooltip>
					<WithTooltip content={t("Borrar nota")}>
						<Button
							size="icon-sm"
							variant="ghost"
							aria-label={t("Borrar nota")}
							onClick={() => void hacer({ que: "quitarNota", id: nota.id })}
						>
							<Icono icon="lucide:trash-2" className="size-3.5" />
						</Button>
					</WithTooltip>
				</span>
			)}
		</div>
	);
}
