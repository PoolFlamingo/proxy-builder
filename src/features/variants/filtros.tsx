import { type Opcion, Selector } from "@/components/campos";
import { useLang } from "@/components/language-provider";
import { Checkbox } from "@/components/ui/checkbox";

/** Idiomas para filtrar búsquedas (`any` = cualquiera). */
export function useIdiomasFiltro(): Opcion<string>[] {
	const { t } = useLang();
	return [
		{ valor: "any", etiqueta: t("Cualquier idioma") },
		{ valor: "en", etiqueta: "English" },
		{ valor: "es", etiqueta: "Español" },
		{ valor: "fr", etiqueta: "Français" },
		{ valor: "de", etiqueta: "Deutsch" },
		{ valor: "it", etiqueta: "Italiano" },
		{ valor: "pt", etiqueta: "Português" },
		{ valor: "ja", etiqueta: "日本語" },
		{ valor: "ko", etiqueta: "한국어" },
		{ valor: "ru", etiqueta: "Русский" },
		{ valor: "zhs", etiqueta: "简体中文" },
		{ valor: "zht", etiqueta: "繁體中文" },
	];
}

export function FiltrosBusqueda({
	lang,
	onLang,
	altaRes,
	onAltaRes,
}: {
	lang: string;
	onLang: (v: string) => void;
	altaRes: boolean;
	onAltaRes: (v: boolean) => void;
}) {
	const { t } = useLang();
	const idiomas = useIdiomasFiltro();
	return (
		<div className="flex flex-wrap items-center gap-2.5">
			<Selector
				valor={lang}
				opciones={idiomas}
				onChange={onLang}
				ancho="w-40"
				ariaLabel={t("Idioma")}
			/>
			<label className="flex items-center gap-2 text-xs">
				<Checkbox checked={altaRes} onCheckedChange={(v) => onAltaRes(v === true)} />
				{t("Solo alta resolución")}
			</label>
		</div>
	);
}

/** `any` → `null` (lo que espera Rust). */
export function idiomaParaRust(lang: string): string | null {
	return lang === "any" ? null : lang;
}
