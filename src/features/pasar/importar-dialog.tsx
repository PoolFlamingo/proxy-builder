import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { PaqueteLeido } from "@/bindings/PaqueteLeido";
import { DialogoAdaptable } from "@/components/dialogo-adaptable";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogCuerpo,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { useBiblioteca } from "@/features/library/store";
import { call } from "@/lib/ipc";
import { abrirDeck, type Devolver, prestarDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import {
	aDecisiones,
	type Elegido,
	eleccionesIniciales,
	puedeResponder,
	quedaAlgo,
	reemplazados,
	textoDeError,
} from "./cuentas";
import { useBajarImagenes } from "./imagenes";
import { OfertaDecks } from "./oferta";
import { ProgresoImagenes } from "./piezas";
import { usePasarUi } from "./store";

/**
 * **Importar un `.pbdeck`** (el selector o «Abrir con…» desde otra app): enseña lo que trae y pregunta lo mismo que al
 * recibir de otro equipo —con proxies o solo las cartas originales, y si ya
 * tienes ese deck, reemplazarlo o guardar los dos—. Rust lo lee sin importar
 * (`paquete_leer`) y lo importa con lo elegido (`paquete_importar`).
 */
export function DialogoImportar() {
	const d = usePasarUi((s) => (s.dialogo?.tipo === "importar" ? s.dialogo : null));
	const cerrar = usePasarUi((s) => s.cerrar);
	return (
		<Dialog open={d !== null} onOpenChange={(o) => !o && cerrar()}>
			{d && <Contenido key={d.ruta} ruta={d.ruta} onCerrar={cerrar} />}
		</Dialog>
	);
}

function Contenido({ ruta, onCerrar }: { ruta: string; onCerrar: () => void }) {
	const { t, mensaje } = useLang();
	const [leido, setLeido] = useState<PaqueteLeido | null>(null);
	const [fallo, setFallo] = useState<string | null>(null);
	const [elegidos, setElegidos] = useState<Elegido[]>([]);
	const [importando, setImportando] = useState(false);
	const [hecho, setHecho] = useState<{ id: string; nombre: string } | null>(null);
	const { avance, fallidas, pedir } = useBajarImagenes();

	useEffect(() => {
		let vivo = true;
		call("paquete_leer", { ruta })
			.then((p) => {
				if (!vivo) return;
				setLeido(p);
				setElegidos(eleccionesIniciales([{ deck: p.deck, conflicto: p.conflicto }]));
			})
			.catch((e) => vivo && setFallo(textoDeError(e, t, mensaje)));
		return () => {
			vivo = false;
		};
	}, [ruta, t, mensaje]);

	const oferta = leido ? [{ deck: leido.deck, conflicto: leido.conflicto }] : [];
	const listo = leido !== null && puedeResponder(oferta, elegidos) && quedaAlgo(elegidos);

	const importar = async () => {
		if (!leido) return;
		const [dec] = aDecisiones(oferta, elegidos);
		if (!dec || dec.eleccion === "no") return;
		setImportando(true);
		// Si reemplaza el deck abierto: lo pendiente se guarda antes y, al
		// acabar, la pantalla lo vuelve a leer de Rust (sin pisarlo).
		const [reemplaza] = reemplazados(oferta, elegidos);
		let devolver: Devolver | null = null;
		try {
			if (reemplaza) devolver = await prestarDeck(reemplaza, { conservar: false });
			const r = await call("paquete_importar", {
				ruta,
				eleccion: dec.eleccion,
				resolucion: dec.resolucion,
			});
			await useBiblioteca.getState().refrescar();
			setHecho({ id: r.deckId, nombre: r.nombre });
			if (r.faltanImagenes > 0) pedir([r.deckId]);
			else
				toast.success(t("«{nombre}» importado", { nombre: r.nombre }), {
					action: { label: t("Abrir"), onClick: () => void abrir(r.deckId) },
				});
		} catch (e) {
			toast.error(textoDeError(e, t, mensaje));
		} finally {
			await devolver?.();
			setImportando(false);
		}
	};
	const abrir = async (id: string) => {
		try {
			await abrirDeck(id);
			useUi.getState().setVista("deck");
			onCerrar();
		} catch (e) {
			toast.error(mensaje(e));
		}
	};

	return (
		<DialogoAdaptable className="text-xs sm:max-w-lg" cambiosSinGuardar={false}>
			<DialogHeader>
				<DialogTitle>{t("Importar un deck")}</DialogTitle>
				<DialogDescription className="text-xs">
					{t("Elige qué quieres guardar de este fichero.")}
				</DialogDescription>
			</DialogHeader>
			<DialogCuerpo>
				{fallo ? (
					<p className="p-1 text-destructive">{fallo}</p>
				) : !leido ? (
					<div className="flex items-center justify-center gap-2 p-6 text-muted-foreground">
						<Icono icon="lucide:loader-2" className="size-4 animate-spin" />
						{t("Leyendo el fichero…")}
					</div>
				) : hecho ? (
					<div className="flex flex-col gap-2 p-1">
						<p className="font-medium">{t("«{nombre}» importado", { nombre: hecho.nombre })}</p>
						<ProgresoImagenes avance={avance} fallidas={fallidas} />
					</div>
				) : (
					<OfertaDecks
						oferta={oferta}
						elegidos={elegidos}
						onCambio={(i, e) => setElegidos((v) => v.map((x, j) => (j === i ? e : x)))}
					/>
				)}
			</DialogCuerpo>
			<DialogFooter>
				<Button size="sm" variant="outline" onClick={onCerrar}>
					{hecho || fallo ? t("Cerrar") : t("Cancelar")}
				</Button>
				{hecho ? (
					<Button size="sm" onClick={() => void abrir(hecho.id)} disabled={avance !== null}>
						{t("Abrir")}
					</Button>
				) : (
					<Button size="sm" onClick={() => void importar()} disabled={!listo || importando}>
						{importando && <Icono icon="lucide:loader-2" className="animate-spin" />}
						{t("Importar")}
					</Button>
				)}
			</DialogFooter>
		</DialogoAdaptable>
	);
}
