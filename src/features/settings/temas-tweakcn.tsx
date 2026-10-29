import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { TemaCatalogo } from "@/bindings/TemaCatalogo";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { useTheme } from "@/components/theme-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogCuerpo,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { esAppError } from "@/lib/errores";
import { call } from "@/lib/ipc";
import { esTemaInstalado, useTemasInstalados } from "@/lib/temas-instalados";
import { THEMES, type ThemeName } from "@/lib/themes";
import { cn } from "@/lib/utils";

/** El tema de fábrica cuando el elegido ya no está instalado. */
const DE_FABRICA: ThemeName = "amber-minimal";

/**
 * Pide a Rust los temas instalados al arrancar y, si el elegido ya no está
 * (quitado o borrado a mano), vuelve al de fábrica. Va montado en `App`.
 */
export function SincronizarTemas() {
	const { theme, setTheme } = useTheme();
	const temas = useTemasInstalados((s) => s.temas);
	const cargar = useTemasInstalados((s) => s.cargar);
	useEffect(() => {
		void cargar();
	}, [cargar]);
	useEffect(() => {
		if (temas && esTemaInstalado(theme) && !temas.some((t) => t.id === theme)) {
			setTheme(DE_FABRICA);
		}
	}, [temas, theme, setTheme]);
	return null;
}

/** Una muestra del tema con sus propias variables (sin aplicarlo a la app). */
function Muestra({ tema }: { tema: TemaCatalogo }) {
	const { t } = useLang();
	const { resolvedMode } = useTheme();
	return (
		<>
			{/* Solo variables comprobadas por Rust (`temas.rs`), con su propio selector. */}
			<style>{tema.css}</style>
			<div
				data-theme={`tw-${tema.id}`}
				className={cn(
					"flex flex-col gap-2.5 rounded-xl border bg-background p-2.5 font-sans text-foreground",
					resolvedMode === "dark" && "dark",
				)}
			>
				<div className="flex flex-col gap-1 rounded-lg border bg-card p-2.5 text-card-foreground">
					<span className="text-sm font-semibold">{tema.nombre}</span>
					<span className="text-xs text-muted-foreground">
						{tema.letra
							? t("Letra: {letra}", { letra: tema.letra })
							: t("Con las letras de la app")}
					</span>
				</div>
				<div className="flex flex-wrap items-center gap-2">
					<span className="rounded-md bg-primary px-2.5 py-1 text-xs text-primary-foreground">
						{t("Primario")}
					</span>
					<span className="rounded-md bg-secondary px-2.5 py-1 text-xs text-secondary-foreground">
						{t("Secundario")}
					</span>
					<span className="rounded-md bg-accent px-2.5 py-1 text-xs text-accent-foreground">
						{t("Acento")}
					</span>
					<span className="rounded-md bg-destructive px-2.5 py-1 text-xs text-white">
						{t("Peligro")}
					</span>
					<span className="rounded-md bg-muted px-2.5 py-1 text-xs text-muted-foreground">
						{t("Apagado")}
					</span>
				</div>
			</div>
		</>
	);
}

/**
 * **Más temas de tweakcn**: los del catálogo oficial que la app aún no trae y
 * cualquiera de la comunidad por su enlace (tweakcn no tiene un listado
 * público de la comunidad: se abre su galería en el navegador y se pega aquí
 * el enlace del tema). Rust lo baja con sus letras; aquí se ve, se instala y
 * se usa.
 */
export function TemasTweakcn({ abierto, onCerrar }: { abierto: boolean; onCerrar: () => void }) {
	const { t, mensaje } = useLang();
	const { theme, setTheme } = useTheme();
	const instalados = useTemasInstalados((s) => s.temas) ?? [];
	const instalar = useTemasInstalados((s) => s.instalar);
	const quitar = useTemasInstalados((s) => s.quitar);
	const [enlace, setEnlace] = useState("");
	const [viendo, setViendo] = useState<TemaCatalogo | null>(null);
	const [catalogo, setCatalogo] = useState<TemaCatalogo[] | null>(null);
	const [ocupado, setOcupado] = useState<"ver" | "instalar" | null>(null);

	useEffect(() => {
		if (!abierto) return;
		setCatalogo(null);
		call("themes_catalog", {})
			// Solo los que la app no trae de serie.
			.then((c) => setCatalogo(c.filter((x) => !THEMES.some((s) => s.name === x.id))))
			.catch(() => setCatalogo([]));
	}, [abierto]);

	const ver = async () => {
		if (!enlace.trim()) return;
		setOcupado("ver");
		try {
			setViendo(await call("theme_fetch", { id: enlace }));
		} catch (e) {
			if (esAppError(e) && e.params.motivo === "tema_enlace") {
				toast.error(t("Eso no es un enlace ni un id de un tema de tweakcn."));
			} else if (esAppError(e) && e.code === "NotFound") {
				toast.error(t("tweakcn no tiene ningún tema con ese enlace."));
			} else toast.error(mensaje(e));
		} finally {
			setOcupado(null);
		}
	};

	const instalarYUsar = async (tema: TemaCatalogo) => {
		setOcupado("instalar");
		try {
			const i = await instalar(tema.id);
			setTheme(i.id as ThemeName);
			toast.success(t("Tema «{nombre}» instalado", { nombre: i.nombre }));
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setOcupado(null);
		}
	};

	const quitarTema = async (id: string) => {
		try {
			if (theme === id) setTheme(DE_FABRICA);
			await quitar(id);
		} catch (e) {
			toast.error(mensaje(e));
		}
	};

	const yaInstalado = (tema: TemaCatalogo) => instalados.some((i) => i.id === `tw-${tema.id}`);

	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && !ocupado && onCerrar()}>
			<DialogContent className="sm:max-w-2xl" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Más temas de tweakcn")}</DialogTitle>
					<DialogDescription>
						{t(
							"Busca un tema en la galería de la comunidad de tweakcn, copia su enlace y pégalo aquí. Se descarga con sus letras y queda en la lista de temas.",
						)}
					</DialogDescription>
				</DialogHeader>
				<DialogCuerpo className="min-h-64">
					<div className="flex flex-col gap-2.5 p-1">
						<div className="flex gap-2">
							<Input
								value={enlace}
								onChange={(e) => setEnlace(e.target.value)}
								onKeyDown={(e) => e.key === "Enter" && void ver()}
								placeholder={t("Enlace o id del tema en tweakcn")}
								className="h-8 text-xs"
							/>
							<Button size="sm" onClick={ver} disabled={!enlace.trim() || ocupado !== null}>
								{ocupado === "ver" ? (
									<Icono icon="lucide:loader-2" className="animate-spin" />
								) : (
									<Icono icon="lucide:eye" />
								)}
								{t("Ver")}
							</Button>
							<Button
								size="sm"
								variant="outline"
								onClick={() =>
									call("themes_open_community", {}).catch((e) => toast.error(mensaje(e)))
								}
							>
								<Icono icon="lucide:external-link" />
								{t("Abrir la galería")}
							</Button>
						</div>

						{viendo && (
							<div className="flex flex-col gap-2">
								<Muestra tema={viendo} />
								<div className="flex justify-end">
									<Button
										size="sm"
										onClick={() => instalarYUsar(viendo)}
										disabled={ocupado !== null}
									>
										{ocupado === "instalar" ? (
											<Icono icon="lucide:loader-2" className="animate-spin" />
										) : (
											<Icono icon="lucide:download" />
										)}
										{yaInstalado(viendo) ? t("Volver a descargar y usar") : t("Instalar y usar")}
									</Button>
								</div>
							</div>
						)}

						{catalogo && catalogo.length > 0 && (
							<section className="flex flex-col gap-1.5">
								<h3 className="text-xs font-semibold">{t("Nuevos en el catálogo de tweakcn")}</h3>
								<div className="grid grid-cols-2 gap-2 p-1 sm:grid-cols-3">
									{catalogo.map((c) => (
										<button
											key={c.id}
											type="button"
											onClick={() => setViendo(c)}
											className={cn(
												"soft-hover flex items-center gap-2 rounded-md border px-2.5 py-2 text-left",
												viendo?.id === c.id && "border-primary ring-1 ring-primary",
											)}
										>
											<span className="flex shrink-0">
												{c.oscuro.slice(0, 3).map((color) => (
													<span
														key={color}
														className="-ml-1 size-3.5 rounded-full border first:ml-0"
														style={{ background: color }}
													/>
												))}
											</span>
											<span className="truncate text-xs font-medium">{c.nombre}</span>
										</button>
									))}
								</div>
							</section>
						)}

						<section className="flex flex-col gap-1.5">
							<h3 className="text-xs font-semibold">{t("Instalados")}</h3>
							{instalados.length === 0 ? (
								<p className="text-xs text-muted-foreground">{t("Aún no hay ninguno.")}</p>
							) : (
								<ul className="flex flex-col gap-1 p-1">
									{instalados.map((i) => (
										<li
											key={i.id}
											className="flex items-center gap-2 rounded-md border px-2.5 py-1.5"
										>
											<span
												className="size-3.5 shrink-0 rounded-full border"
												style={{ background: i.muestra }}
											/>
											<span className="truncate text-xs font-medium">{i.nombre}</span>
											<span className="ml-auto flex items-center gap-1">
												{theme === i.id ? (
													<Icono icon="lucide:check" className="size-3.5 text-primary" />
												) : (
													<Button
														size="sm"
														variant="ghost"
														onClick={() => setTheme(i.id as ThemeName)}
													>
														{t("Usar")}
													</Button>
												)}
												<WithTooltip content={t("Quitar")}>
													<Button
														size="icon"
														variant="ghost"
														className="size-7"
														onClick={() => quitarTema(i.id)}
														aria-label={t("Quitar")}
													>
														<Icono icon="lucide:trash-2" />
													</Button>
												</WithTooltip>
											</span>
										</li>
									))}
								</ul>
							)}
						</section>
					</div>
				</DialogCuerpo>
				<DialogFooter>
					<Button size="sm" variant="outline" onClick={onCerrar} disabled={ocupado !== null}>
						{t("Cerrar")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
