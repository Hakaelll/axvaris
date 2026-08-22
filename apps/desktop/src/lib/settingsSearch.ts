export const settingsSearchIndex = [
  {
    section: "capture",
    label: "Capture",
    keywords:
      "capture captura interface interfaz startup inicio handshake metadatos profile perfil",
  },
  {
    section: "focus",
    label: "Focus rules",
    keywords: "focus enfoque bookmarks marcadores ignore ignorar filtros",
  },
  {
    section: "notifications",
    label: "Notifications",
    keywords:
      "notifications notificaciones quiet hours silencio ASN port puerto",
  },
  {
    section: "privacy",
    label: "Privacy & storage",
    keywords:
      "privacy privacidad storage almacenamiento retention retención ephemeral efímero export redaction borrado",
  },
  {
    section: "geo",
    label: "GeoIP data",
    keywords: "geoip location ubicación mmdb city ciudad asn database datos",
  },
  {
    section: "display",
    label: "Display",
    keywords:
      "display visual pantalla presentation presentación pseudonyms seudónimos language idioma density densidad contrast contraste text scale zoom 200",
  },
  {
    section: "diagnostics",
    label: "Diagnostics",
    keywords:
      "diagnostics diagnóstico health salud cpu memory memoria backend counters contadores support soporte",
  },
] as const;

export function findSettingsSections(query: string) {
  const normalized = query.trim().toLocaleLowerCase();
  if (!normalized) return [];
  return settingsSearchIndex.filter((item) =>
    `${item.label} ${item.keywords}`.toLocaleLowerCase().includes(normalized),
  );
}
