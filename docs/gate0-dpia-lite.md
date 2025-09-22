# Gate 0 DPIA-lite – Delta 1 Generieke AI-assistent

## 1. Verwerkingsactiviteiten
- **Doel:** leveren van AI-ondersteuning voor medewerkers en klanten met informatievoorziening, samenvattingen en automatisering.
- **Betrokken systemen:** AI-assistent services (`delta1_*`), CRM/ticketing, interne kennisbanken, consentstore, monitoring/logging.
- **Gegevensbronnen:** publieke documentatie, interne beleidsdocumenten en FAQ’s, metadata van interacties, CRM-ticketdata.

## 2. Categorieën persoonsgegevens
- Identificatiegegevens: naam, e-mailadres, telefoonnummer, klantnummer.
- Contacthistorie: ticketcontext en vrije tekst (mogelijk PII, incidenteel gevoelige data zoals gezondheid of financiën).
- Gebruiksmetadata: tijdstip, kanaal, type vraag, tool calls.

## 3. Rechtsgrond en consent
- **Lawful basis:** contractuele noodzaak voor bestaande klanten en expliciete opt-in via CMP voor overige gebruikers.
- **Consentbeheer:** consent-banner/opt-in beheerd via CMP (bijv. OneTrust); synchronisatie naar centrale consentstore via API-calls; AI-assistent raadpleegt store real-time of via periodieke sync.
- **NoConsent-beleid:** `DeltaCode::NoConsent` blokkeert toegang tot interne data en levert generieke antwoorden; gemapt naar HTTP 403 met auditlogging.

## 4. Risicoanalyse
| Risico | Impact | Waarschijnlijkheid | Mitigaties |
| --- | --- | --- | --- |
| Onbedoelde verwerking van PII/gevoelige data via vrije tekst | Hoog | Middel | DLP-detectie, automatische redactie, verplichte HITL bij detectie, dataminimalisatie. |
| Ongeautoriseerde toegang tot interne documenten | Hoog | Laag | Rolgebaseerde toegang, sandboxing, allowlists, rate limiting, audittrail. |
| Datalek via logs of monitoring | Middel | Laag | Logging zonder inhoud (alleen metadata), 24-uurs purge van tijdelijke velden, encryptie in rust en transit. |
| Onverklaarbare beslissingen / gebrek aan uitlegbaarheid | Middel | Middel | WhyLog-coverage, human override, escalatie naar HITL bij twijfel, documentatie van beslissingen. |
| Fairness-regressie / bias | Middel | Middel | CI-gates op fairness/differential privacy, periodieke modelcard updates, monitoring van beslissingsscores. |
| Operationele drift / modelveroudering | Middel | Middel | Continue monitoring, versiebeheer, automatische mitigaties en rollback naar veilige modellen. |

## 5. Dataretentie en verwijdering
- Ruwe logs maximaal 30 dagen; gevoelige velden zo snel mogelijk purgen, streefwaarde < 24 uur.
- Geanonimiseerde interactiestatistieken maximaal 12 maanden voor trendanalyse.
- Consentstatussen en audittrails worden behouden volgens wettelijke bewaartermijnen (minimaal 1 jaar, maximaal 7 jaar afhankelijk van sectorregels).

## 6. Beveiligingsmaatregelen
- Sandboxing van agent-acties en gecontroleerde toolaanroepen via ToolRegistry.
- Rate limiting en commandbudget-controle om misbruik te voorkomen.
- Toegangsbeheer geïntegreerd met bestaande IAM-oplossingen.
- Monitoring op policy-overtredingen, escalaties en consent-violations.
- Incidentresponsplan gekoppeld aan ticketingsysteem met correlatie-ID’s voor herleidbaarheid zonder inhoudelijke logging.

## 7. Betrokkenenrechten
- Ondersteuning voor inzage-, correctie- en verwijderingsverzoeken via CRM/ticketing.
- Consentstore houdt gehashte subject-id’s bij om verzoeken efficiënt af te handelen.
- DeltaCode-logica documenteert geweigerde verzoeken zodat het bezwaarrecht kan worden nageleefd.

## 8. HITL en governance
- Escalatiepad naar 1e lijn support en 2e lijn domeinspecialisten; eindverantwoordelijkheid bij afdelingsmanager.
- Ticketingsysteem (Jira Service Management, ServiceNow of Zendesk) vormt de primaire tooling; specialistensystemen (ITSM, HR-portal, juridische tools) voor tweede lijn.
- Audit trail via ticket-ID waarin AI-actie en menselijke beoordeling worden vastgelegd; versies en statuswissels worden automatisch bijgehouden.

## 9. Openstaande acties voor Gate 0
- Formele goedkeuring van DPO en CIO/CTO op dit DPIA-lite document.
- Juridische validatie van consentmechanisme en retentiontermijnen.
- Implementatie en test van DLP-filters op vrije tekst voordat pilot start.
- Voorbereiden van `DeltaCode::NoConsent` testscripts en documentatie van resultaten.
