import { Component, type ReactNode } from "react";

import { traduccionSuelta as t } from "@/components/language-provider";
import { Button } from "@/components/ui/button";

/**
 * Red de seguridad de la interfaz: sin esto, un error de render deja la
 * ventana **en blanco** y nadie sabe si ha perdido su deck. Aquí se dice qué
 * ha pasado y se ofrece volver. Nunca sustituye a arreglar la causa.
 */
export class LimiteError extends Component<{ children: ReactNode }, { error: Error | null }> {
	state: { error: Error | null } = { error: null };

	static getDerivedStateFromError(error: Error) {
		return { error };
	}

	componentDidCatch(error: Error, info: unknown) {
		console.error("Proxy Builder — error de interfaz:", error, info);
	}

	render() {
		if (!this.state.error) return this.props.children;
		return (
			<div className="grid h-svh place-items-center bg-background p-6 text-foreground">
				<div className="flex max-w-md flex-col items-center gap-3 text-center">
					<h1 className="text-sm font-semibold">{t("Algo ha fallado en la pantalla")}</h1>
					<p className="text-xs text-muted-foreground">
						{t(
							"Tus decks están a salvo: viven en la biblioteca local, no en esta pantalla. Puedes volver e intentarlo de nuevo.",
						)}
					</p>
					<pre className="max-h-40 w-full overflow-auto rounded-md border bg-muted/40 p-2 text-left text-[11px] text-muted-foreground">
						{this.state.error.message}
					</pre>
					<div className="flex gap-2">
						<Button size="sm" onClick={() => this.setState({ error: null })}>
							{t("Volver")}
						</Button>
						<Button size="sm" variant="outline" onClick={() => window.location.reload()}>
							{t("Recargar la ventana")}
						</Button>
					</div>
				</div>
			</div>
		);
	}
}
