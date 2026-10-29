import { type ReactNode, useState } from "react";
import { CampoNumero } from "@/components/campos";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { useTheme } from "@/components/theme-provider";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { THEMES } from "@/lib/themes";
import { cn } from "@/lib/utils";
import { TemasTweakcn } from "./temas-tweakcn";

/** Tamaño de la interfaz, en %: de 5 en 5 entre estos topes. */
const ESCALA_MIN = 70;
const ESCALA_MAX = 150;

function SettingCard({
	title,
	desc,
	span,
	headerRight,
	children,
}: {
	title: string;
	desc?: string;
	span?: boolean;
	headerRight?: ReactNode;
	children: ReactNode;
}) {
	return (
		<Card className={cn("gap-3 py-4", span && "md:col-span-2")}>
			<CardHeader className="px-4">
				<CardTitle className="text-sm">{title}</CardTitle>
				{(desc || headerRight) && (
					<div className="entra flex items-center justify-between gap-2">
						<CardDescription className="text-xs">{desc}</CardDescription>
						{headerRight}
					</div>
				)}
			</CardHeader>
			<CardContent className="px-4">{children}</CardContent>
		</Card>
	);
}

function MiniCheck({
	checked,
	onChange,
	label,
}: {
	checked: boolean;
	onChange: (b: boolean) => void;
	label: string;
}) {
	return (
		<button
			type="button"
			onClick={() => onChange(!checked)}
			className="flex shrink-0 items-center gap-1.5 text-[11px] text-muted-foreground"
		>
			<span
				className={cn(
					"flex size-3.5 items-center justify-center rounded-[3px] border transition-colors",
					checked ? "border-primary bg-primary text-primary-foreground" : "border-input",
				)}
			>
				{checked && <Icono icon="lucide:check" className="size-2.5" />}
			</span>
			{label}
		</button>
	);
}

function LabeledRange({
	label,
	value,
	display,
	min,
	max,
	step,
	onChange,
	disabled,
}: {
	label: string;
	value: number;
	display: string;
	min: number;
	max: number;
	step: number;
	onChange: (n: number) => void;
	disabled?: boolean;
}) {
	return (
		<div className={cn("flex flex-col gap-1.5", disabled && "opacity-40")}>
			<div className="flex items-center justify-between gap-2 text-[11px] uppercase tracking-wide text-muted-foreground">
				<span>{label}</span>
				<span className="tabular-nums">{display}</span>
			</div>
			<input
				type="range"
				min={min}
				max={max}
				step={step}
				value={value}
				disabled={disabled}
				onChange={(e) => onChange(Number(e.target.value))}
				className="macup-range w-full cursor-pointer disabled:cursor-not-allowed"
			/>
		</div>
	);
}

export function Appearance() {
	const {
		theme,
		setTheme,
		mode,
		setMode,
		scale,
		setScale,
		skin,
		setSkin,
		bg,
		setBg,
		material,
		setMaterial,
		blur,
		setBlur,
		noise,
		setNoise,
		shadow,
		setShadow,
		fxSidebar,
		setFxSidebar,
		heraFiligree,
		setHeraFiligree,
		smoothScroll,
		setSmoothScroll,
	} = useTheme();
	const { t } = useLang();
	const [masTemas, setMasTemas] = useState(false);

	const shadowNames = [t("Ninguna"), t("Suave"), t("Normal"), t("Fuerte"), t("Muy fuerte")];

	return (
		<div className="flex w-full flex-col gap-4">
			<div className="grid grid-cols-1 gap-2.5 md:grid-cols-2">
				<SettingCard title={t("Apariencia")} desc={t("Claro, oscuro o el del sistema.")}>
					<ToggleGroup
						type="single"
						variant="outline"
						size="sm"
						value={mode}
						onValueChange={(v) => v && setMode(v as typeof mode)}
					>
						<ToggleGroupItem value="light" className="gap-1.5 px-3 text-xs">
							<Icono icon="lucide:sun" className="size-3.5" />
							{t("Claro")}
						</ToggleGroupItem>
						<ToggleGroupItem value="dark" className="gap-1.5 px-3 text-xs">
							<Icono icon="lucide:moon" className="size-3.5" />
							{t("Oscuro")}
						</ToggleGroupItem>
						<ToggleGroupItem value="system" className="gap-1.5 px-3 text-xs">
							<Icono icon="lucide:monitor" className="size-3.5" />
							{t("Sistema")}
						</ToggleGroupItem>
					</ToggleGroup>
				</SettingCard>

				<SettingCard
					title={t("Tamaño de la interfaz")}
					desc={t("Escala toda la app.")}
					headerRight={
						<MiniCheck
							checked={smoothScroll}
							onChange={setSmoothScroll}
							label={t("Scroll suave")}
						/>
					}
				>
					<div className="flex items-center gap-1.5">
						<Button
							size="icon-sm"
							variant="outline"
							className="size-7"
							disabled={Math.round(scale * 100) <= ESCALA_MIN}
							onClick={() => setScale(Math.max(ESCALA_MIN, Math.round(scale * 100) - 5) / 100)}
							aria-label={t("Reducir")}
						>
							<Icono icon="lucide:minus" />
						</Button>
						<CampoNumero
							valor={Math.round(scale * 100)}
							onChange={(v) => setScale((v ?? 100) / 100)}
							min={ESCALA_MIN}
							max={ESCALA_MAX}
							paso={5}
							sufijo="%"
							ancho="w-20"
							ariaLabel={t("Tamaño de la interfaz")}
						/>
						<Button
							size="icon-sm"
							variant="outline"
							className="size-7"
							disabled={Math.round(scale * 100) >= ESCALA_MAX}
							onClick={() => setScale(Math.min(ESCALA_MAX, Math.round(scale * 100) + 5) / 100)}
							aria-label={t("Ampliar")}
						>
							<Icono icon="lucide:plus" />
						</Button>
						<Button size="sm" variant="ghost" className="h-7 text-xs" onClick={() => setScale(1)}>
							100 %
						</Button>
					</div>
				</SettingCard>

				<SettingCard
					title={t("Estilo de superficie")}
					desc={t("Zeus: relieve 3D. Hera: diseño de cristal flotante.")}
				>
					{/* Con el tema Base los efectos están apagados a propósito (ver
              `ThemeProvider`). Si no se dijera aquí, quien pulsara Hera
              vería que no pasa nada y pensaría que está roto. */}
					{theme === "base" && (
						<p className="entra mb-2 rounded-lg border border-amber-500/40 bg-amber-500/10 px-2.5 py-1.5 text-[11px] leading-snug text-muted-foreground">
							{t("Con el tema")} <strong className="text-foreground">Base</strong>{" "}
							{t(
								"esto no se aplica: Base se pinta tal cual sale de shadcn, sin cristal, sin fondo y sin sombras, para poder comparar contra él. Lo que elija aquí se guarda y vuelve en cuanto cambie de tema.",
							)}
						</p>
					)}
					<div className="flex flex-wrap items-center gap-3">
						<ToggleGroup
							type="single"
							variant="outline"
							size="sm"
							value={skin}
							onValueChange={(v) => v && setSkin(v as typeof skin)}
						>
							<ToggleGroupItem value="none" className="px-3 text-xs">
								{t("Plano")}
							</ToggleGroupItem>
							<ToggleGroupItem value="zeus" className="px-3 text-xs">
								Zeus
							</ToggleGroupItem>
							<ToggleGroupItem value="hera" className="px-3 text-xs">
								Hera
							</ToggleGroupItem>
						</ToggleGroup>
						{skin === "hera" && (
							<MiniCheck checked={heraFiligree} onChange={setHeraFiligree} label={t("Filigrana")} />
						)}
					</div>
				</SettingCard>

				<SettingCard title={t("Fondo de pantalla")} desc={t("Para el estilo Plano.")}>
					<ToggleGroup
						type="single"
						variant="outline"
						size="sm"
						value={bg}
						onValueChange={(v) => v && setBg(v as typeof bg)}
					>
						<ToggleGroupItem value="none" className="px-3 text-xs">
							{t("Liso")}
						</ToggleGroupItem>
						<ToggleGroupItem value="gradient" className="px-3 text-xs">
							{t("Degradado")}
						</ToggleGroupItem>
						<ToggleGroupItem value="aurora" className="px-3 text-xs">
							Aurora
						</ToggleGroupItem>
					</ToggleGroup>
				</SettingCard>

				<SettingCard
					title={t("Material de las tarjetas")}
					desc={t("Sólido, cristal o cristal mate con grano.")}
				>
					<ToggleGroup
						type="single"
						variant="outline"
						size="sm"
						value={material}
						onValueChange={(v) => v && setMaterial(v as typeof material)}
					>
						<ToggleGroupItem value="solid" className="px-3 text-xs">
							{t("Sólido")}
						</ToggleGroupItem>
						<ToggleGroupItem value="glass" className="px-3 text-xs">
							{t("Cristal")}
						</ToggleGroupItem>
						<ToggleGroupItem value="matte" className="px-3 text-xs">
							{t("Mate")}
						</ToggleGroupItem>
					</ToggleGroup>
				</SettingCard>

				<SettingCard
					title={t("Efectos")}
					desc={t("Desenfoque, grano y sombra.")}
					headerRight={
						<MiniCheck
							checked={fxSidebar}
							onChange={setFxSidebar}
							label={t("Incluye los paneles")}
						/>
					}
				>
					<div className="grid grid-cols-3 gap-3">
						<LabeledRange
							label={t("Desenfoque")}
							value={blur}
							display={`${blur}`}
							min={0}
							max={40}
							step={1}
							onChange={setBlur}
							disabled={material === "solid"}
						/>
						<LabeledRange
							label={t("Grano")}
							value={noise}
							display={`${Math.round(noise * 100)}`}
							min={0}
							max={0.5}
							step={0.01}
							onChange={setNoise}
							disabled={material !== "matte"}
						/>
						<LabeledRange
							label={t("Sombra")}
							value={shadow}
							display={shadowNames[shadow] ?? t("Normal")}
							min={0}
							max={4}
							step={1}
							onChange={setShadow}
						/>
					</div>
				</SettingCard>

				<SettingCard
					title={t("Tema de color")}
					span
					headerRight={
						<Button size="sm" variant="outline" onClick={() => setMasTemas(true)}>
							<Icono icon="lucide:palette" />
							{t("Más temas…")}
						</Button>
					}
				>
					<div className="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4">
						{/* `tema` y no `t`: `t` es ya la función de traducción de esta
                pantalla, y volver a llamar así al tema la tapaba dentro del
                bucle. */}
						{THEMES.map((tema) => (
							<button
								key={tema.name}
								onClick={() => setTheme(tema.name)}
								className={cn(
									"soft-hover relative flex items-center gap-2 rounded-md border px-2.5 py-2 text-left transition-colors",
									theme === tema.name && "border-primary ring-1 ring-primary",
								)}
							>
								<span
									className="size-4 shrink-0 rounded-full border"
									style={{ background: tema.swatch }}
								/>
								<span className="truncate text-xs font-medium">{tema.label}</span>
								{theme === tema.name && (
									<Icono icon="lucide:check" className="ml-auto size-3.5 shrink-0 text-primary" />
								)}
							</button>
						))}
					</div>
				</SettingCard>
			</div>
			<TemasTweakcn abierto={masTemas} onCerrar={() => setMasTemas(false)} />
		</div>
	);
}
