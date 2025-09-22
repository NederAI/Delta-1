# Gate 0 Purpose- en Consentregistratie – Delta 1 Generieke AI-assistent

## 1. Overzicht
Deze notitie documenteert de purpose-id’s, consentflows en testscenario’s die nodig zijn om Gate 0 af te ronden voor de generieke AI-assistent.

## 2. Purpose-id’s
| Purpose-id | Type | Beschrijving | Datacategorieën | Verantwoordelijke |
| --- | --- | --- | --- | --- |
| `AI_Assist_Generic_Info` | Nieuw | Leveren van generieke antwoorden en publieke documentatie aan gebruikers zonder interne context. | Publieke kennis, metadata van interacties. | Product owner AI-assistent. |
| `AI_Assist_Internal_Productivity` | Uitbreiding | Ondersteunen van medewerkers bij interne processen (samenvattingen, rapportages, Q&A) op basis van interne documenten. | Interne beleidsstukken, FAQ’s, CRM-ticketcontext, metadata. | Afdelingsmanager / proceseigenaren. |

## 3. Consentbeheer
- **CMP-integratie:** Consent-banner/opt-in wordt beheerd via bestaand CMP (OneTrust of interne module).
- **Synchronisatie:** CMP schrijft consentstatus naar centrale consentstore via API; opslag bevat `subject_id` (gehasht), `purpose_id`, status, timestamp en bron.
- **AI-consumptie:** AI-assistent raadpleegt consentstore real-time (API) of via periodieke sync; fallback-cache maximaal 24 uur geldig.
- **Audit:** Elke wijziging in consentstatus wordt gelogd met request-ID en ticket-ID voor traceerbaarheid.

## 4. DeltaCode mapping en testen
| DeltaCode | HTTP-status | Beschrijving | Actie |
| --- | --- | --- | --- |
| `Ok` | 200 | Volledige functionaliteit toegestaan. | Normale afhandeling en logging zonder inhoud. |
| `HitlRequired` | 202 | Menselijke beoordeling vereist. | Maak/actualiseer ticket; wijs toe aan 1e lijn support. |
| `NoConsent` | 403 | Geen geldige consent voor aangevraagde purpose. | Lever alleen generieke antwoorden, log weigering en ticket voor follow-up. |
| `InvalidPurpose` | 422 | Purpose-id niet erkend. | Fout teruggeven, escaleer naar product owner. |
| `NotFound` | 404 | Gevraagde bron niet beschikbaar. | Meld aan gebruiker, log voor incidentanalyse. |
| `InvalidRequest` | 400 | Ongeldige input. | Geef fout terug, overweeg DLP-scan. |
| `InternalError` | 500 | Onverwachte fout. | Escalatie naar 2e lijn, log met correlatie-ID. |

- **Testscenario’s:**
  1. Consent aanwezig voor beide purpose-id’s → verwachte `DeltaCode::Ok` en HTTP 200.
  2. Consent ontbreekt of ingetrokken → `DeltaCode::NoConsent`, HTTP 403, generieke fallback zonder interne data.
  3. Purpose onbekend → `DeltaCode::InvalidPurpose`, HTTP 422, auditverplichting.
  4. HITL-trigger (gevoelige context) → `DeltaCode::HitlRequired`, HTTP 202 met ticketaanmaak.
- **Meetpunten:** aantal NoConsent-gevallen, responstijd CMP-check, succesratio van consent-sync jobs.

## 5. HITL- en auditkoppeling
- **Ticketing:** 1e lijn verwerkt escalaties in systeem zoals Jira Service Management, ServiceNow of Zendesk; 2e lijn in specialistische systemen (ITSM, HR, juridisch).
- **Audit trail:** Elke escalatie krijgt ticket-ID met log van AI-actie, consentstatus en menselijke beslissing; wijzigingen worden automatisch vastgelegd door ticketingsysteem.

## 6. Openstaande acties
- Implementatie van productieklare consentstore (vervang `AllowAllConsent`).
- Validatie van CMP-API-koppeling en fallbackmechanisme.
- Documenteer en automatiseer `DeltaCode::NoConsent` testscripts.
- Bevestig bewaartermijnen voor consentlogs conform sectorregels (1–7 jaar).
