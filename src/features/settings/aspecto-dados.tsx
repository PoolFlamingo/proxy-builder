import { Dices, ImagePlus, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import type { AspectoDados } from "@/bindings/AspectoDados";
import { Fila, FilaDeslizador, FilaInterruptor, Selector } from "@/components/campos";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	ASPECTO_DE_FABRICA,
	claveEstilo,
	estiloDe,
	useAspectoDados,
} from "@/features/mesa/dados/aspecto";
import type { Tirada } from "@/features/mesa/dados/cola";
import { DadosCss } from "@/features/mesa/dados/css3d";
import { lienzoFondo, urlFondo } from "@/features/mesa/dados/texturas";
import { abrirParaImportar } from "@/lib/dialogos";
import { useUi } from "@/stores/ui-store";

/** El color de muestra cuando el dado lleva el del jugador. */
const COLOR_DE_MUESTRA = "#7c3aed";

/** Una tirada de muestra: un d20 en 20, un d6 en 6 y una moneda de cara. */
function muestra(semilla: number): Tirada {
	return {
		id: semilla,
		actor: null,
		semilla,
		piezas: [
			{ forma: "dado", tipo: 20, valor: 20 },
			{ forma: "dado", tipo: 6, valor: 6 },
			{ forma: "moneda", cara: true },
		],
		cambio: { tipo: "monedas", lanzamientos: [[true]], hastaPerder: false, semillaAnimacion: 0 },
	};
}

/**
 * **Ajustes > Mesa > Dados y monedas**: el aspecto de tus dados y tu moneda
 * (`mesa/dados.rs`) con una muestra en CSS 3D que se tira con «Probar», y el
 * sonido. Los colores se guardan al soltar (un momento después de cambiar).
 */
export function AjustesDados() {
	const { t, mensaje } = useLang();
	const m = useUi((s) => s.mesa);
	const set = useUi((s) => s.setMesa);
	const vista = useAspectoDados((s) => s.vista);
	const [local, setLocal] = useState<AspectoDados>(vista?.aspecto ?? ASPECTO_DE_FABRICA);
	const [semilla, setSemilla] = useState(1);
	const [sonar, setSonar] = useState(false);
	const [ocupado, setOcupado] = useState(false);
	const espera = useRef<number | undefined>(undefined);

	useEffect(() => {
		void useAspectoDados.getState().cargar();
	}, []);
	useEffect(() => {
		if (vista) setLocal(vista.aspecto);
	}, [vista]);
	useEffect(() => () => window.clearTimeout(espera.current), []);

	const cambiar = (c: Partial<AspectoDados>) => {
		const nuevo = { ...local, ...c };
		setLocal(nuevo);
		window.clearTimeout(espera.current);
		espera.current = window.setTimeout(() => {
			useAspectoDados
				.getState()
				.poner(nuevo)
				.catch((e) => toast.error(mensaje(e)));
		}, 250);
	};

	const imagen = async (quitar: boolean) => {
		let ruta: string | null = null;
		if (!quitar) {
			const rutas = await abrirParaImportar({
				multiple: false,
				filters: [{ name: t("Imágenes"), extensions: ["png", "jpg", "jpeg", "webp"] }],
			});
			if (!rutas?.[0]) return;
			ruta = rutas[0];
		}
		setOcupado(true);
		try {
			await useAspectoDados.getState().ponerImagen(ruta);
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setOcupado(false);
		}
	};

	const estilo = useMemo(
		() =>
			estiloDe(
				{ aspecto: local, imagen: local.imagen ? (vista?.imagen ?? null) : null },
				COLOR_DE_MUESTRA,
				true,
			),
		[local, vista],
	);
	const tirada = useMemo(() => muestra(semilla), [semilla]);
	const clave = claveEstilo(estilo);
	const [fondo, setFondo] = useState<{ clave: string; url: string } | null>(null);
	useEffect(() => {
		let vivo = true;
		void lienzoFondo(estilo).then((c) => {
			if (vivo) setFondo({ clave, url: urlFondo(c) });
		});
		return () => {
			vivo = false;
		};
	}, [estilo, clave]);

	return (
		<>
			<h4 className="text-[11px] font-semibold text-muted-foreground uppercase">
				{t("Dados y monedas")}
			</h4>
			<div className="relative h-52 overflow-hidden rounded-xl border bg-muted/30">
				{fondo && (
					<DadosCss
						key={semilla}
						tirada={tirada}
						estilo={estilo}
						fondo={fondo.url}
						alAcabar={() => {}}
						sonido={sonar}
					/>
				)}
				<Button
					size="sm"
					variant="outline"
					className="absolute right-2.5 bottom-2.5"
					onClick={() => {
						setSonar(true);
						setSemilla((s) => s + 1);
					}}
				>
					<Dices className="size-3.5" />
					{t("Probar")}
				</Button>
			</div>
			<FilaInterruptor
				etiqueta={t("Del color del jugador")}
				valor={local.color === null}
				onChange={(v) => cambiar({ color: v ? null : COLOR_DE_MUESTRA })}
			/>
			{local.color !== null && (
				<Fila etiqueta={t("Color del dado")}>
					<input
						type="color"
						value={local.color}
						onChange={(e) => cambiar({ color: e.target.value })}
						aria-label={t("Color del dado")}
						className="h-7 w-12 cursor-pointer rounded-md border bg-transparent p-0.5"
					/>
				</Fila>
			)}
			<Fila etiqueta={t("Color de los números")}>
				<input
					type="color"
					value={local.numeros}
					onChange={(e) => cambiar({ numeros: e.target.value })}
					aria-label={t("Color de los números")}
					className="h-7 w-12 cursor-pointer rounded-md border bg-transparent p-0.5"
				/>
			</Fila>
			<Fila etiqueta={t("Acabado")}>
				<Selector<AspectoDados["acabado"]>
					valor={local.acabado}
					onChange={(v) => cambiar({ acabado: v })}
					opciones={[
						{ valor: "liso", etiqueta: t("Liso") },
						{ valor: "nacarado", etiqueta: t("Nacarado") },
						{ valor: "marmol", etiqueta: t("Mármol") },
						{ valor: "purpurina", etiqueta: t("Purpurina") },
						{ valor: "translucido", etiqueta: t("Translúcido") },
					]}
				/>
			</Fila>
			<Fila
				etiqueta={t("Imagen propia")}
				ayuda={t("Va en cada cara, debajo de los números, en lugar del color y el acabado.")}
			>
				<div className="flex items-center gap-1.5">
					{vista?.imagen && local.imagen && (
						<img src={vista.imagen} alt="" className="size-7 rounded-md border object-cover" />
					)}
					<Button size="sm" variant="outline" disabled={ocupado} onClick={() => void imagen(false)}>
						<ImagePlus className="size-3.5" />
						{t("Elegir…")}
					</Button>
					{local.imagen && (
						<Button
							size="sm"
							variant="ghost"
							disabled={ocupado}
							onClick={() => void imagen(true)}
							aria-label={t("Quitar la imagen")}
						>
							<X className="size-3.5" />
						</Button>
					)}
				</div>
			</Fila>
			<Fila etiqueta={t("Moneda")}>
				<Selector<AspectoDados["moneda"]>
					valor={local.moneda}
					onChange={(v) => cambiar({ moneda: v })}
					opciones={[
						{ valor: "oro", etiqueta: t("Oro") },
						{ valor: "plata", etiqueta: t("Plata") },
						{ valor: "cobre", etiqueta: t("Cobre") },
					]}
				/>
			</Fila>
			<FilaInterruptor
				etiqueta={t("Sonido de los dados y las monedas")}
				valor={m.sonidoDados}
				onChange={(v) => set({ sonidoDados: v })}
			/>
			{m.sonidoDados && (
				<FilaDeslizador
					etiqueta={t("Volumen")}
					valor={m.volumen}
					onChange={(v) => set({ volumen: v })}
					min={0}
					max={100}
					paso={5}
					sufijo=" %"
				/>
			)}
		</>
	);
}
