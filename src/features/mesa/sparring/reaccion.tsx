import { Ghost } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { Decision } from "@/bindings/Decision";
import type { IdCarta } from "@/bindings/IdCarta";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { claveZona, useMesa } from "@/stores/mesa-store";
import { hacer } from "../acciones";
import { CartaFantasma } from "./carta-fantasma";
import { lineaBase, porQue, titulo } from "./textos";

const NINGUNA: IdCarta[] = [];

function Tecla({ children }: { children: React.ReactNode }) {
	return (
		<kbd className="rounded border border-current/40 px-1 font-mono text-[10px] opacity-80">
			{children}
		</kbd>
	);
}

/**
 * **La reacción del fantasma** (`capturas/playgroup/90-spar-prompt-4.jpg`):
 * qué hace y con qué carta, «¿Por qué ahora?», «Ver el porqué» y las tres
 * respuestas (Intro, A, Esc). Mientras espera respuesta, los atajos de la
 * mesa no van (es un `role="dialog"`). En un sacrificio, «Resolverla» pide
 * antes qué criatura se sacrifica.
 */
export function ReaccionPendiente() {
	const { t } = useLang();
	const pendiente = useMesa(
		(s) => s.general?.sparring.reacciones.find((r) => r.decision === null) ?? null,
	);
	const yo = useMesa((s) => s.yo ?? 0);
	const campo = useMesa(
		(s) => s.zonas[claveZona({ tipo: "de", jugador: yo, zona: "campo" })]?.cartas ?? NINGUNA,
	);
	const cartas = useMesa((s) => s.cartas);
	const [verBase, setVerBase] = useState(false);
	const [eligiendo, setEligiendo] = useState(false);
	const id = pendiente?.reaccion.id ?? null;
	useEffect(() => {
		if (id !== null) {
			setVerBase(false);
			setEligiendo(false);
		}
	}, [id]);

	const decidir = (decision: Decision, eleccion: IdCarta | null = null) => {
		if (id === null) return;
		void hacer({ que: "decidirSparring", reaccion: id, decision, eleccion });
	};
	const resolver = () => {
		if (pendiente?.reaccion.tipo === "sacrificio") setEligiendo(true);
		else decidir("aceptar");
	};
	const teclas = useRef({ resolver, decidir, eligiendo });
	teclas.current = { resolver, decidir, eligiendo };

	useEffect(() => {
		if (id === null) return;
		const alTeclear = (e: KeyboardEvent) => {
			const el = e.target as HTMLElement | null;
			if (el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable)) {
				return;
			}
			// Otro diálogo encima (el de «Cómo funciona»): ese manda.
			if (document.querySelectorAll('[role="dialog"]').length > 1) return;
			const k = teclas.current;
			const tecla = e.key.toLowerCase();
			if (tecla === "enter" && !k.eligiendo) k.resolver();
			else if (tecla === "a" && !k.eligiendo) k.decidir("teniaRespuesta");
			else if (tecla === "escape") {
				if (k.eligiendo) setEligiendo(false);
				else k.decidir("noAplica");
			} else return;
			e.preventDefault();
			e.stopImmediatePropagation();
		};
		window.addEventListener("keydown", alTeclear, true);
		return () => window.removeEventListener("keydown", alTeclear, true);
	}, [id]);

	if (!pendiente) return null;
	const r = pendiente.reaccion;
	const objetivo =
		r.objetivo[0] !== undefined ? (cartas[r.objetivo[0]]?.origen?.nombre ?? t("una carta")) : "";
	const criaturas = campo.filter((c) => {
		const v = cartas[c];
		return v && (v.bocaAbajo || v.origen?.info.esCriatura);
	});

	return (
		<div
			role="dialog"
			aria-modal="false"
			aria-label={t("Rival fantasma")}
			className="absolute top-16 left-1/2 z-[8500] flex w-[22rem] -translate-x-1/2 flex-col gap-2.5 rounded-xl border border-primary/50 bg-card/95 p-3 text-xs shadow-2xl backdrop-blur-md animate-in fade-in zoom-in-95"
		>
			<div className="flex items-center gap-1.5 text-[10px] font-semibold tracking-wide text-primary uppercase">
				<Ghost className="size-3.5" />
				{t("Rival fantasma")}
			</div>
			<h3 className="text-sm font-semibold">{titulo(r, objetivo, t)}</h3>
			<div className="flex items-start gap-2.5">
				<CartaFantasma nombre={r.carta.nombre} ancho={488} className="w-36 shrink-0" />
				<div className="flex min-w-0 flex-col gap-1.5">
					<p className="font-medium">
						{r.carta.nombre}{" "}
						<span className="rounded border px-1 font-mono text-[10px] text-muted-foreground">
							{r.carta.coste}
						</span>
					</p>
					<h4 className="text-[10px] font-semibold text-primary uppercase">
						{t("¿Por qué ahora?")}
					</h4>
					<p className="text-foreground/90">{porQue(r.porQue, t)}</p>
					<button
						type="button"
						onClick={() => setVerBase((v) => !v)}
						className="self-start text-[10px] font-semibold text-primary uppercase underline-offset-2 hover:underline"
					>
						{verBase ? t("Ocultar el porqué") : t("Ver el porqué")}
					</button>
				</div>
			</div>
			{verBase && (
				<ul className="flex list-disc flex-col gap-0.5 rounded-md border bg-muted/40 py-1.5 pr-2 pl-5 text-muted-foreground">
					{r.base.map((b) => (
						<li key={b.clave}>{lineaBase(b, t)}</li>
					))}
				</ul>
			)}
			{eligiendo ? (
				<div className="flex flex-col gap-1.5">
					<p className="font-medium">{t("Elige la criatura que sacrificas")}</p>
					<ScrollArea viewportClassName="max-h-40">
						<div className="flex flex-col gap-1 p-1">
							{criaturas.map((c) => (
								<Button
									key={c}
									size="sm"
									variant="outline"
									className="justify-start"
									onClick={() => decidir("aceptar", c)}
								>
									{cartas[c]?.origen?.nombre ?? t("una carta")}
								</Button>
							))}
						</div>
					</ScrollArea>
					<Button size="sm" variant="ghost" onClick={() => setEligiendo(false)}>
						{t("Cancelar")} <Tecla>Esc</Tecla>
					</Button>
				</div>
			) : (
				<div className="flex flex-col gap-1.5">
					<Button size="sm" onClick={resolver}>
						{t("Resolverla")} <Tecla>{t("Intro")}</Tecla>
					</Button>
					<Button size="sm" variant="outline" onClick={() => decidir("teniaRespuesta")}>
						{t("Tengo respuesta")} <Tecla>A</Tecla>
					</Button>
					<Button size="sm" variant="ghost" onClick={() => decidir("noAplica")}>
						{t("No aplica")} <Tecla>Esc</Tecla>
					</Button>
				</div>
			)}
		</div>
	);
}
