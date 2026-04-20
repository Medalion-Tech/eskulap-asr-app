use crate::ast::{
    deterministic_slot_id, FilledTemplate, FilledValue, Inline, Node, PickOption, Slot, SlotId,
    TemplateAst,
};
use crate::templates::Template;
use std::collections::BTreeMap;

const BUILTIN_TIMESTAMP: &str = "2026-04-20 00:00:00";

pub fn all() -> Vec<Template> {
    vec![poz_visit(), imaging_chest(), discharge_v2()]
}

// ---------- Helpers ----------

struct Builder {
    template_id: String,
    nodes: Vec<Node>,
    slots: BTreeMap<SlotId, Slot>,
}

impl Builder {
    fn new(template_id: &str) -> Self {
        Self {
            template_id: template_id.to_string(),
            nodes: Vec::new(),
            slots: BTreeMap::new(),
        }
    }

    fn sid(&self, name: &str) -> SlotId {
        deterministic_slot_id(&self.template_id, name)
    }

    fn heading(mut self, level: u8, text: &str) -> Self {
        self.nodes.push(Node::Heading {
            level,
            inlines: vec![Inline::Text { text: text.to_string() }],
        });
        self
    }

    fn para_text(mut self, text: &str) -> Self {
        self.nodes.push(Node::Paragraph {
            inlines: vec![Inline::Text { text: text.to_string() }],
        });
        self
    }

    /// A paragraph like `label<slot>` — label text followed by one inline slot.
    fn labeled_field(mut self, label: &str, name: &str, hint: Option<&str>) -> Self {
        let id = self.sid(name);
        self.slots.insert(
            id.clone(),
            Slot::Field {
                id: id.clone(),
                name: name.to_string(),
                hint: hint.map(|s| s.to_string()),
                default: None,
            },
        );
        self.nodes.push(Node::Paragraph {
            inlines: vec![
                Inline::Text { text: label.to_string() },
                Inline::Slot { id },
            ],
        });
        self
    }

    fn labeled_pick(
        mut self,
        label: &str,
        name: &str,
        options: &[(&str, &str)],
        allow_other: bool,
    ) -> Self {
        let id = self.sid(name);
        let mut opts: Vec<PickOption> = options
            .iter()
            .map(|(c, t)| PickOption { code: c.to_string(), text: t.to_string() })
            .collect();
        // Always ensure X (nieokreślone) is present.
        if !opts.iter().any(|o| o.code == "X") {
            opts.push(PickOption { code: "X".to_string(), text: "nieokreślone".to_string() });
        }
        self.slots.insert(
            id.clone(),
            Slot::Pick {
                id: id.clone(),
                name: name.to_string(),
                hint: None,
                options: opts,
                allow_other,
            },
        );
        self.nodes.push(Node::Paragraph {
            inlines: vec![
                Inline::Text { text: label.to_string() },
                Inline::Slot { id },
            ],
        });
        self
    }

    fn longtext_block(mut self, name: &str, hint: Option<&str>) -> Self {
        let id = self.sid(name);
        self.slots.insert(
            id.clone(),
            Slot::Longtext {
                id: id.clone(),
                name: name.to_string(),
                hint: hint.map(|s| s.to_string()),
            },
        );
        self.nodes.push(Node::SlotBlock { id });
        self
    }

    fn list_block(mut self, name: &str, hint: Option<&str>, numbered: bool) -> Self {
        let id = self.sid(name);
        self.slots.insert(
            id.clone(),
            Slot::List {
                id: id.clone(),
                name: name.to_string(),
                hint: hint.map(|s| s.to_string()),
                numbered,
            },
        );
        self.nodes.push(Node::SlotBlock { id });
        self
    }

    fn build(self) -> TemplateAst {
        TemplateAst {
            nodes: self.nodes,
            slots: self.slots,
        }
    }
}

fn builtin(
    id: &str,
    name: &str,
    description: &str,
    ast: TemplateAst,
    example_input: &str,
    example_filled: FilledTemplate,
) -> Template {
    Template {
        id: id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        ast,
        example_input: Some(example_input.to_string()),
        example_filled: Some(example_filled),
        is_builtin: true,
        created_at: BUILTIN_TIMESTAMP.to_string(),
        updated_at: BUILTIN_TIMESTAMP.to_string(),
        ast_version: 1,
    }
}

fn sid(template_id: &str, name: &str) -> SlotId {
    deterministic_slot_id(template_id, name)
}

fn text_val(s: &str) -> FilledValue {
    FilledValue::Text { text: s.to_string() }
}

fn pick_val(code: &str) -> FilledValue {
    FilledValue::Pick { code: code.to_string(), custom_text: None }
}

fn list_val(items: &[&str]) -> FilledValue {
    FilledValue::List { items: items.iter().map(|s| s.to_string()).collect() }
}

// ---------- POZ: wizyta kontrolna ----------

fn poz_visit() -> Template {
    let tid = "builtin-poz-visit";
    let ast = Builder::new(tid)
        .heading(1, "Wizyta POZ")
        .labeled_field("Powód wizyty: ", "powod_wizyty", Some("1 zdanie"))
        .heading(2, "Wywiad")
        .para_text("Objawy główne:")
        .longtext_block("objawy_glowne", Some("czas trwania, charakter, dynamika"))
        .para_text("Leki przewlekłe:")
        .list_block("leki_przewlekle", Some("nazwa, dawka, częstość"), false)
        .labeled_field("Alergie: ", "alergie", Some("leki, pokarmy; brak = \"nie zgłasza\""))
        .heading(2, "Badanie przedmiotowe")
        .labeled_pick(
            "Stan ogólny: ",
            "stan_ogolny",
            &[("A", "dobry"), ("B", "średni"), ("C", "ciężki")],
            true,
        )
        .labeled_field("RR: ", "cisnienie", Some("np. 130/80 mmHg"))
        .labeled_field("Tętno: ", "tetno", Some("bpm"))
        .labeled_field("Temperatura: ", "temperatura", Some("°C"))
        .para_text("Badanie fizykalne:")
        .longtext_block("badanie_fizykalne", Some("tylko systemy wymienione przez lekarza"))
        .heading(2, "Rozpoznanie")
        .labeled_field("Rozpoznanie główne: ", "rozpoznanie_glowne", Some("z [ICD-10]"))
        .para_text("Rozpoznania współistniejące:")
        .list_block("rozpoznania_wspolistniejace", Some("z [ICD-10]"), false)
        .heading(2, "Zalecenia")
        .para_text("Farmakoterapia:")
        .list_block("farmakoterapia", Some("nazwa, dawka, częstość, czas"), true)
        .para_text("Badania dodatkowe:")
        .list_block("badania_dodatkowe", Some("laboratoryjne, obrazowe"), false)
        .labeled_field("Kontrola za: ", "kontrola_za", Some("np. 7 dni"))
        .para_text("Czerwone flagi:")
        .longtext_block("czerwone_flagi", Some("kiedy pilnie do SOR"))
        .build();

    let example_input = "Pacjentka czterdzieści pięć lat, zgłasza się z bólem kręgosłupa lędźwiowego od trzech dni. Ból pojawił się po podnoszeniu cięższego przedmiotu, nasila się przy pochylaniu, łagodzi pozycja leżąca. Nie promieniuje do nóg. Choroby przewlekłe: nadciśnienie, bierze ramipril pięć miligramów. Alergii nie zgłasza. W badaniu stan ogólny dobry. Ciśnienie sto trzydzieści na osiemdziesiąt, tętno siedemdziesiąt dwa, temperatura trzydzieści sześć i sześć. Bolesność palpacyjna okolicy lędźwiowej, napięcie mięśni przykręgosłupowych, Laseque ujemny obustronnie. Rozpoznaję zespół bólowy kręgosłupa lędźwiowego. Daję naproksen pięćset dwa razy dziennie przez pięć dni, tyzanidyna dwa miligramy wieczorem. Zalecam unikanie dźwigania. Kontrola za tydzień. Pilna wizyta przy drętwieniu nóg lub zaburzeniach oddawania moczu.";

    let mut values = BTreeMap::new();
    values.insert(sid(tid, "powod_wizyty"), text_val("ból kręgosłupa L/S od 3 dni"));
    values.insert(
        sid(tid, "objawy_glowne"),
        text_val("Ból pojawił się po podnoszeniu ciężkiego przedmiotu, nasila się przy pochylaniu, łagodzi pozycja leżąca. Bez promieniowania do kończyn dolnych."),
    );
    values.insert(
        sid(tid, "leki_przewlekle"),
        list_val(&["ramipril 5 mg 1×/d"]),
    );
    values.insert(sid(tid, "alergie"), text_val("nie zgłasza"));
    values.insert(sid(tid, "stan_ogolny"), pick_val("A"));
    values.insert(sid(tid, "cisnienie"), text_val("130/80 mmHg"));
    values.insert(sid(tid, "tetno"), text_val("72 bpm"));
    values.insert(sid(tid, "temperatura"), text_val("36,6°C"));
    values.insert(
        sid(tid, "badanie_fizykalne"),
        text_val("Bolesność palpacyjna okolicy lędźwiowej, wzmożone napięcie mięśni przykręgosłupowych. Objaw Lasègue'a ujemny obustronnie."),
    );
    values.insert(
        sid(tid, "rozpoznanie_glowne"),
        text_val("Zespół bólowy kręgosłupa odcinka lędźwiowego [ICD-10: M54.5]"),
    );
    values.insert(
        sid(tid, "rozpoznania_wspolistniejace"),
        list_val(&["Nadciśnienie tętnicze [ICD-10: I10]"]),
    );
    values.insert(
        sid(tid, "farmakoterapia"),
        list_val(&[
            "Naproksen 500 mg 2×/d przez 5 dni (po posiłku)",
            "Tyzanidyna 2 mg wieczorem",
        ]),
    );
    values.insert(sid(tid, "badania_dodatkowe"), FilledValue::Unfilled);
    values.insert(sid(tid, "kontrola_za"), text_val("7 dni"));
    values.insert(
        sid(tid, "czerwone_flagi"),
        text_val("Pilna wizyta w razie drętwienia kończyn dolnych, osłabienia siły mięśniowej, zaburzeń oddawania moczu lub stolca."),
    );

    let example_filled = FilledTemplate {
        template_id: tid.to_string(),
        values,
        user_edited: Vec::new(),
    };

    builtin(
        tid,
        "Wizyta POZ — kontrola",
        "Wizyta rodzinna (POZ): wywiad, badanie, rozpoznanie, zalecenia",
        ast,
        example_input,
        example_filled,
    )
}

// ---------- Radiologia: TK/MR klatki piersiowej ----------

fn imaging_chest() -> Template {
    let tid = "builtin-imaging-chest";
    let ast = Builder::new(tid)
        .heading(1, "Opis badania obrazowego klatki piersiowej")
        .labeled_pick(
            "Typ badania: ",
            "typ_badania",
            &[
                ("A", "TK bez kontrastu"),
                ("B", "TK z kontrastem"),
                ("C", "MR"),
            ],
            true,
        )
        .labeled_field("Wskazanie: ", "wskazanie", None)
        .labeled_field("Porównanie z dnia: ", "porownanie_z_dnia", Some("data lub \"brak\""))
        .heading(2, "Opis")
        .para_text("Płuco prawe:")
        .longtext_block("pluca_prawe", None)
        .para_text("Płuco lewe:")
        .longtext_block("pluca_lewe", None)
        .labeled_pick(
            "Opłucna: ",
            "oplucna",
            &[("A", "bez zmian"), ("B", "wysięk"), ("C", "odma")],
            true,
        )
        .para_text("Śródpiersie:")
        .longtext_block("srodpiersie", Some("węzły chłonne, struktury naczyniowe"))
        .labeled_pick(
            "Serce: ",
            "serce_wielkosc",
            &[("A", "prawidłowej wielkości"), ("B", "powiększone")],
            true,
        )
        .para_text("Kości (kręgosłup, żebra, mostek):")
        .longtext_block("kosci", None)
        .heading(2, "Wnioski")
        .list_block("wnioski", Some("od najistotniejszego"), true)
        .para_text("Sugerowana dalsza diagnostyka:")
        .longtext_block("dalsza_diagnostyka", Some("opcjonalnie"))
        .build();

    let example_input = "TK klatki piersiowej bez kontrastu, wskazanie przewlekły kaszel. Bez badania porównawczego. Płuco prawe prawidłowe upowietrznienie, w segmencie tylnym płata górnego drobny lity guzek około sześć milimetrów, niespecyficzny. Płuco lewe bez zmian ogniskowych. Opłucna bez wysięku i bez odmy. Śródpiersie bez powiększenia węzłów chłonnych, aorta i pień płucny prawidłowe. Serce prawidłowej wielkości. Kości bez zmian ogniskowych. Wnioski: drobny guzek płuca prawego do obserwacji. Kontrolne TK za trzy miesiące zgodnie z protokołem Fleischnera.";

    let mut values = BTreeMap::new();
    values.insert(sid(tid, "typ_badania"), pick_val("A"));
    values.insert(sid(tid, "wskazanie"), text_val("przewlekły kaszel"));
    values.insert(sid(tid, "porownanie_z_dnia"), text_val("brak"));
    values.insert(
        sid(tid, "pluca_prawe"),
        text_val("Prawidłowe upowietrznienie. W segmencie tylnym płata górnego drobny lity guzek ok. 6 mm, niespecyficzny."),
    );
    values.insert(sid(tid, "pluca_lewe"), text_val("Bez zmian ogniskowych."));
    values.insert(sid(tid, "oplucna"), pick_val("A"));
    values.insert(
        sid(tid, "srodpiersie"),
        text_val("Bez powiększenia węzłów chłonnych. Aorta i pień płucny o prawidłowej szerokości."),
    );
    values.insert(sid(tid, "serce_wielkosc"), pick_val("A"));
    values.insert(sid(tid, "kosci"), text_val("Bez zmian ogniskowych."));
    values.insert(
        sid(tid, "wnioski"),
        list_val(&[
            "Drobny guzek płuca prawego (~6 mm) — do obserwacji.",
            "Poza tym klatka piersiowa bez istotnych zmian.",
        ]),
    );
    values.insert(
        sid(tid, "dalsza_diagnostyka"),
        text_val("Kontrolne TK klatki piersiowej za 3 miesiące zgodnie z protokołem Fleischnera."),
    );

    let example_filled = FilledTemplate {
        template_id: tid.to_string(),
        values,
        user_edited: Vec::new(),
    };

    builtin(
        tid,
        "TK/MR klatki piersiowej",
        "Strukturalny opis badania obrazowego klatki piersiowej",
        ast,
        example_input,
        example_filled,
    )
}

// ---------- Szpital: karta wypisowa ----------

fn discharge_v2() -> Template {
    let tid = "builtin-discharge-v2";
    let ast = Builder::new(tid)
        .heading(1, "Karta wypisowa")
        .labeled_field("Rozpoznanie główne: ", "rozpoznanie_glowne", Some("z [ICD-10]"))
        .para_text("Rozpoznania współistniejące:")
        .list_block("rozpoznania_wspolistniejace", Some("z [ICD-10]"), false)
        .para_text("Procedury wykonane:")
        .list_block("procedury_wykonane", Some("z [ICD-9]"), false)
        .heading(2, "Epikryza")
        .para_text("Powód hospitalizacji:")
        .longtext_block("powod_hospitalizacji", Some("1-2 zdania"))
        .para_text("Stan przy przyjęciu:")
        .longtext_block("stan_przy_przyjeciu", None)
        .para_text("Kluczowe badania diagnostyczne:")
        .longtext_block("kluczowe_badania", Some("lab, obrazowe"))
        .para_text("Przebieg leczenia:")
        .longtext_block("przebieg_leczenia", Some("narracyjnie, jeden akapit"))
        .labeled_pick(
            "Stan przy wypisie: ",
            "stan_przy_wypisie",
            &[("A", "dobry"), ("B", "średni"), ("C", "ciężki")],
            true,
        )
        .heading(2, "Zalecenia")
        .para_text("Farmakoterapia:")
        .list_block("farmakoterapia", Some("nazwa, dawka, częstość, czas"), true)
        .para_text("Dieta i aktywność:")
        .longtext_block("dieta_aktywnosc", None)
        .para_text("Opieka nad raną:")
        .longtext_block("opieka_nad_rana", Some("jeśli dotyczy"))
        .para_text("Kontrole:")
        .list_block("kontrole", Some("gdzie, kiedy, w jakim celu"), true)
        .para_text("Skierowania:")
        .list_block("skierowania", None, false)
        .para_text("Red flags (pilne do SOR):")
        .longtext_block("red_flags", None)
        .labeled_field(
            "Niezdolność do pracy: ",
            "niezdolnosc_do_pracy_dni",
            Some("liczba dni lub \"nie orzeczono\""),
        )
        .build();

    let example_input = "Pacjent sześćdziesiąt dwa lata, przyjęty w trybie ostrym z SOR z powodu ostrego bólu prawego dolnego kwadranta brzucha od osiemnastu godzin, z nudnościami. Przy przyjęciu obrona mięśniowa w prawym dole biodrowym, Blumberg dodatni. Leukocytoza piętnaście tysięcy, CRP dziewięćdziesiąt osiem. Tomografia potwierdziła ostre zapalenie wyrostka. W trybie pilnym appendektomia laparoskopowa, przebieg niepowikłany. Pooperacyjnie ceftriakson i metronidazol przez trzy doby, profilaktyka enoksaparyną. Pierwsza doba gorączka trzydzieści osiem pięć, potem bezgorączkowy. Rana goi się prawidłowo. Wypis w stanie dobrym. Ciprofloksacyna pięćset dwa razy dziennie i metronidazol pięćset trzy razy dziennie przez pięć dni. Dieta lekkostrawna, oszczędzający tryb życia dwa tygodnie. Wymiana opatrunku co drugi dzień, zdjęcie szwów za dziesięć dni u lekarza rodzinnego. Kontrola w poradni chirurgicznej za dwa tygodnie. Skierowanie do rehabilitacji nie jest potrzebne. Niezdolność do pracy czternaście dni. Pilny powrót do SOR w razie gorączki powyżej trzydziestu ośmiu pięć lub nasilającego się bólu brzucha.";

    let mut values = BTreeMap::new();
    values.insert(
        sid(tid, "rozpoznanie_glowne"),
        text_val("Ostre zapalenie wyrostka robaczkowego bez perforacji [ICD-10: K35.80]"),
    );
    values.insert(sid(tid, "rozpoznania_wspolistniejace"), FilledValue::Unfilled);
    values.insert(
        sid(tid, "procedury_wykonane"),
        list_val(&["Appendektomia laparoskopowa [ICD-9: 47.01]"]),
    );
    values.insert(
        sid(tid, "powod_hospitalizacji"),
        text_val("Ostry ból prawego dolnego kwadranta brzucha od 18 godzin, z nudnościami."),
    );
    values.insert(
        sid(tid, "stan_przy_przyjeciu"),
        text_val("Obrona mięśniowa w prawym dole biodrowym, dodatni objaw Blumberga. Leukocytoza 15 tys./µl, CRP 98 mg/l. TK potwierdziła ostre zapalenie wyrostka."),
    );
    values.insert(
        sid(tid, "kluczowe_badania"),
        text_val("Leukocytoza 15 tys./µl, CRP 98 mg/l. TK jamy brzusznej: obraz ostrego zapalenia wyrostka robaczkowego."),
    );
    values.insert(
        sid(tid, "przebieg_leczenia"),
        text_val("W trybie pilnym wykonano appendektomię laparoskopową — przebieg niepowikłany. Pooperacyjnie: ceftriakson i metronidazol przez 3 doby, profilaktyka przeciwzakrzepowa enoksaparyną. W 1. dobie pooperacyjnej gorączka do 38,5°C, następnie bezgorączkowy. Rana operacyjna goi się prawidłowo."),
    );
    values.insert(sid(tid, "stan_przy_wypisie"), pick_val("A"));
    values.insert(
        sid(tid, "farmakoterapia"),
        list_val(&[
            "Ciprofloksacyna 500 mg 2×/d przez 5 dni",
            "Metronidazol 500 mg 3×/d przez 5 dni",
        ]),
    );
    values.insert(
        sid(tid, "dieta_aktywnosc"),
        text_val("Dieta lekkostrawna. Oszczędzający tryb życia przez 2 tygodnie, bez dźwigania >5 kg."),
    );
    values.insert(
        sid(tid, "opieka_nad_rana"),
        text_val("Zmiana opatrunku co 2 dni; szwy do usunięcia w 10. dobie pooperacyjnej u lekarza POZ."),
    );
    values.insert(
        sid(tid, "kontrole"),
        list_val(&["Poradnia chirurgiczna za 2 tygodnie"]),
    );
    values.insert(sid(tid, "skierowania"), FilledValue::Unfilled);
    values.insert(
        sid(tid, "red_flags"),
        text_val("Pilny powrót do SOR w razie gorączki >38,5°C, nasilającego się bólu brzucha, wymiotów lub zaczerwienienia/wycieku z rany."),
    );
    values.insert(sid(tid, "niezdolnosc_do_pracy_dni"), text_val("14 dni"));

    let example_filled = FilledTemplate {
        template_id: tid.to_string(),
        values,
        user_edited: Vec::new(),
    };

    builtin(
        tid,
        "Karta wypisowa",
        "Karta informacyjna z leczenia szpitalnego — strukturalna",
        ast,
        example_input,
        example_filled,
    )
}
