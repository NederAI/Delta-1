# Gate 0 Intakecanvas – Delta 1 Generieke AI-assistent

## 1. Businesscontext en doelmetrieken
- **Use-case / probleem:** Een generieke AI-assistent die medewerkers en klanten ondersteunt met informatie, productiviteit (samenvattingen, rapporten, Q&A) en automatisering van repetitieve taken om werkdruk te verlagen en responstijd te verkorten.
- **Doelmetrieken:**
  - Gemiddelde responstijd < 2 seconden.
  - Gebruikerstevredenheid (CSAT) > 80%.
  - < 5% escalaties naar menselijke support.
- **HITL-escalatiepaden:**
  - 1e lijn: supportmedewerker via de interne operations-console.
  - 2e lijn: domeinspecialisten (IT, HR, juridisch afhankelijk van context) binnen hetzelfde platform.
  - Verantwoordelijke eindbeslisser: afdelingsmanager.

## 2. Data en acties
- **Gegevenscategorieën:**
  - Publieke kennis (documentatie, handleidingen).
  - Bedrijfsinterne documenten (beleidsstukken, FAQ’s).
  - Metadata van interacties (tijdstip, type vraag, kanaal).
  - Persoonsgegevens die gebruikers zelf invoeren in de interne service-portal: naam, e-mailadres, telefoonnummer, medewerker- of klantnummer en context (vrije tekst kan incidenteel PII bevatten).
  - Noodscenario’s: vrije tekst kan gevoelige data (gezondheid, financiële info) bevatten.
- **Agent-acties en tools:**
  - Tekstanalyse en samenvatting.
  - Toegang tot interne kennisbanken via interne API’s.
  - Interactie met interne workflow-services voor opvolging en logging.
  - Geplande toolkoppelingen via ToolRegistry met uitsluitend interne database- en HTTP-adapters.
  - Alle koppelingen blijven binnen Delta 1; er zijn geen externe SaaS-integraties.
- **Retentiebeleid:**
  - Ruwe logs maximaal 30 dagen.
  - Geanonimiseerde interactiestatistieken 12 maanden.
  - Geen blijvende opslag van gevoelige input; noodscenario’s vereisen DLP-filter en fallback naar HITL.

## 3. Logging en monitoring
- **Logging zonder inhoud:** alleen request-ID, timestamp, gebruikte tool/actie, statuscode en response-tijd.
- **Incidentrespons:** correlatie-ID’s koppelen interacties aan de interne service-portal waar inhoud al beveiligd is.
- **Prestatie- en veiligheidsmetriek:** monitor commandbudget, latenties per tool-call, geblokkeerde policy-overtredingen, HITL-escalaties, consent-violations.

## 4. Privacy, risico’s en mitigaties
- **Belangrijkste risico’s:** onbedoelde verwerking van persoonsgegevens, ongeautoriseerde toegang, tool-misbruik, datalek via logs, onverklaarbare beslissingen, fairness-regressie.
- **Mitigaties:** dataminimalisatie, logging zonder inhoud, toegangsbeheer, monitoring, DLP-filter op vrije tekst, sandboxing en allowlists voor tools, rate limiting, verplicht WhyLog-uitleg bij hoge risico’s, 24-uurs purge van tijdelijke velden, CI-gates voor fairness/DP.
- **Toestemming:** expliciete opt-in via de interne consentmodule; consentstatus beschikbaar via centrale consentstore.

## 5. Purpose, consent en DeltaCode
- **Purpose-id’s:**
  - `AI_Assist_Generic_Info` (nieuw).
  - `AI_Assist_Internal_Productivity` (uitbreiding bestaand).
- **Consentflow:** standaard opt-in voor eindgebruikers, consent vastgelegd via interne consentmodule en gesynchroniseerd met consentstore; AI-assistent raadpleegt store real-time of via sync.
- **NoConsent-handling:** `DeltaCode::NoConsent` leidt tot generieke antwoorden zonder interne data en map naar HTTP 403.

## 6. Risicoklassen en externe communicatie
- **Risiconiveaus:**
  - Laag: feitelijke informatie, samenvattingen.
  - Midden: beslissingsondersteuning (HR, IT advies).
  - Hoog: juridische of medische context → altijd HITL.
- **Extern contract:** DeltaCode→HTTP mapping (OK→200, HITL nodig→202, NoConsent→403) aangevuld met bestaande mappings (422, 404, 400, 500) voor fouten.
- **Compliance:** GDPR, EU AI Act (transparantie, risicobeoordeling), sectorregels (zorg, finance).

## 7. Randvoorwaarden en afhankelijkheden
- **Stakeholders Gate 0:** CIO/CTO, DPO, afdelingsmanagers.
- **Benodigde documentatie:** Intakecanvas, DPIA-lite, escalerende HITL-protocollen, purpose-registraties, DeltaCode::NoConsent tests.
- **Afhankelijkheden:** interne API-toegang binnen Delta 1, juridische goedkeuring consentmechanisme, resources voor modelmonitoring, productieklare consentstore in plaats van `AllowAllConsent`.

## 8. Volgende stappen
- Finaliseer DPIA-lite met bovenstaande risico’s en mitigaties.
- Beschrijf HITL-escapepaden in detail en plan training voor 1e/2e lijn.
- Automatische DLP en consentchecks implementeren en testen inclusief `NoConsent`-scenario.
