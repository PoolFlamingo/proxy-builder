import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { Bandera, banderaDe, Emoji, nombreNoto, urlEmoji } from "@/components/emoji";
import { IDIOMAS } from "@/lib/idiomas";

describe("emoji de Noto", () => {
	it("una bandera da su imagen", () => {
		expect(banderaDe("es")).toBe("🇪🇸");
		expect(nombreNoto("🇪🇸")).toBe("emoji_u1f1ea_1f1f8");
		expect(urlEmoji("🇪🇸")).toMatch(/emoji_u1f1ea_1f1f8\.(svg|png)/);
	});

	it("están las banderas de los 15 idiomas", () => {
		for (const i of IDIOMAS) expect(urlEmoji(banderaDe(i.pais)), i.codigo).toBeDefined();
	});

	it("el selector de variante no cuenta", () => {
		expect(nombreNoto("✏️")).toBe("emoji_u270f");
	});

	it("un emoji sin imagen se queda como carácter", () => {
		expect(urlEmoji("🦄")).toBeUndefined();
		expect(renderToStaticMarkup(<Emoji>🦄</Emoji>)).toBe("<span>🦄</span>");
		expect(renderToStaticMarkup(<Bandera pais="JP" alt="日本語" />)).toMatch(
			/<img[^>]*alt="日本語"/,
		);
	});
});
