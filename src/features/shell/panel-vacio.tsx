import { useLang } from "@/components/language-provider";

/** Hueco provisional de un panel mientras se construye su contenido. */
export function PanelVacio({ titulo }: { titulo: string }) {
	const { t } = useLang();
	return (
		<section className="flex h-full flex-col gap-2.5 p-2.5">
			<h2 className="text-[11px] font-semibold uppercase tracking-widest text-muted-foreground">
				{t(titulo)}
			</h2>
		</section>
	);
}
