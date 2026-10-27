import { Coins, Dices } from "lucide-react";
import { useState } from "react";
import { CampoNumero, Selector } from "@/components/campos";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Switch } from "@/components/ui/switch";
import { useMesa } from "@/stores/mesa-store";
import { hacer } from "../acciones";
import { usePantalla } from "../pantalla";

const DADOS = [4, 6, 8, 10, 12, 20] as const;

/**
 * **Dados y monedas** (`capturas/pablo/01.jpg`): monedas (hasta perder, con
 * pulgares de Krark), tiradas rápidas, N dados de X, d100 y el dado
 * planar. El resultado lo decide Rust y sale aquí y en el registro.
 */
export function AzarDialog() {
	const { t } = useLang();
	const [n, setN] = useState(2);
	const [caras, setCaras] = useState<number>(6);
	const [pulgares, setPulgares] = useState(0);
	const [hastaPerder, setHastaPerder] = useState(false);
	const ultimo = useMesa((s) => {
		for (let i = s.registro.length - 1; i >= 0; i--) {
			const c = s.registro[i]?.cambio;
			if (c && (c.tipo === "dados" || c.tipo === "monedas" || c.tipo === "dadoPlanar")) return c;
		}
		return null;
	});
	// Al tirar se cierra: la tirada y su resultado salen sobre la mesa (`dados/capa.tsx`).
	const lanzar = (a: Parameters<typeof hacer>[0]) =>
		void hacer(a).then((ok) => ok && usePantalla.getState().abrir(null));
	const tirar = (dados: [number, number][]) => lanzar({ que: "tirarDados", dados });
	return (
		<Dialog open onOpenChange={(o) => !o && usePantalla.getState().abrir(null)}>
			<DialogContent className="sm:max-w-md" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Dados y monedas")}</DialogTitle>
					<DialogDescription>
						{t("El resultado sale también en el registro de la partida.")}
					</DialogDescription>
				</DialogHeader>
				<ResultadoAzar cambio={ultimo} />
				<section className="flex flex-col gap-1.5">
					<h4 className="text-xs font-semibold">{t("Moneda")}</h4>
					<div className="flex flex-wrap items-center gap-2.5 text-xs">
						<Button
							size="sm"
							onClick={() => lanzar({ que: "moneda", cuantas: 1, hastaPerder, pulgares })}
						>
							<Coins className="size-3.5" />
							{t("Lanzar la moneda")}
						</Button>
						<label className="flex items-center gap-1.5">
							<Switch checked={hastaPerder} onCheckedChange={setHastaPerder} />
							{t("Hasta perder")}
						</label>
						<label className="flex items-center gap-1.5">
							{t("Pulgares de Krark")}
							<CampoNumero
								valor={pulgares}
								onChange={(v) => setPulgares(v ?? 0)}
								min={0}
								max={4}
								paso={1}
								ancho="w-14"
							/>
						</label>
					</div>
				</section>
				<section className="flex flex-col gap-1.5">
					<h4 className="text-xs font-semibold">{t("Tiradas rápidas")}</h4>
					<div className="flex flex-wrap gap-1.5">
						{DADOS.map((d) => (
							<Button key={d} size="sm" variant="outline" onClick={() => tirar([[d, 1]])}>
								d{d}
							</Button>
						))}
						<Button size="sm" variant="outline" onClick={() => tirar([[100, 1]])}>
							d100
						</Button>
						<Button size="sm" variant="outline" onClick={() => lanzar({ que: "dadoPlanar" })}>
							{t("Dado planar")}
						</Button>
					</div>
				</section>
				<section className="flex items-center gap-2.5 text-xs">
					<CampoNumero
						valor={n}
						onChange={(v) => setN(v ?? 1)}
						min={1}
						max={100}
						paso={1}
						ancho="w-16"
						ariaLabel={t("Cuántos dados")}
					/>
					<span>{t("dados de")}</span>
					<Selector<string>
						valor={String(caras)}
						onChange={(v) => setCaras(Number(v))}
						opciones={[...DADOS, 100].map((d) => ({ valor: String(d), etiqueta: `d${d}` }))}
						ancho="w-20"
					/>
					<Button size="sm" onClick={() => tirar([[caras, n]])}>
						<Dices className="size-3.5" />
						{t("Tirar")}
					</Button>
				</section>
			</DialogContent>
		</Dialog>
	);
}

type CambioAzar = Extract<
	import("@/bindings/Cambio").Cambio,
	{ tipo: "dados" } | { tipo: "monedas" } | { tipo: "dadoPlanar" }
>;

/** El último resultado, en grande. */
export function ResultadoAzar({ cambio }: { cambio: CambioAzar | null }) {
	const { t } = useLang();
	if (!cambio) return null;
	if (cambio.tipo === "dados") {
		const suma = cambio.tiradas.reduce((a, x) => a + x.valor, 0);
		return (
			<div className="flex flex-col items-center gap-1 rounded-lg bg-muted/50 p-2.5">
				<div className="flex flex-wrap justify-center gap-1.5">
					{cambio.tiradas.map((x, i) => (
						<span
							key={i}
							className="flex size-10 items-center justify-center rounded-lg bg-background text-lg font-bold shadow"
						>
							{x.valor}
						</span>
					))}
				</div>
				{cambio.tiradas.length > 1 && (
					<span className="text-xs text-muted-foreground">{t("Suma: {n}", { n: suma })}</span>
				)}
			</div>
		);
	}
	if (cambio.tipo === "monedas") {
		return (
			<div className="flex flex-wrap justify-center gap-1.5 rounded-lg bg-muted/50 p-2.5 text-xs">
				{cambio.lanzamientos.map((l, i) => (
					<span key={i} className="rounded-md bg-background px-2 py-1 font-semibold shadow">
						{l.map((x) => (x ? t("Cara") : t("Cruz"))).join(" / ")}
					</span>
				))}
			</div>
		);
	}
	return (
		<div className="rounded-lg bg-muted/50 p-2.5 text-center text-sm font-semibold">
			{cambio.cara === "planeswalk"
				? t("Planeswalk")
				: cambio.cara === "caos"
					? t("Caos")
					: t("Nada")}
			<span className="ml-2 text-xs font-normal text-muted-foreground">
				{t("Esta tirada costaba {n}", { n: cambio.coste })}
			</span>
		</div>
	);
}
