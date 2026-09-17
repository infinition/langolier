// English source text -> French
export const FR: Record<string, string> = {
  "Open the Langolier app to reach the Rust engine. This page is an interface preview.":
    "Ouvrez l’application Langolier pour utiliser le moteur Rust. Cette page est un aperçu de l’interface.",
  Enter: "Entrée",
  Esc: "Échap",
  "LM Studio / llama.cpp (local OpenAI)":
    "LM Studio / llama.cpp (OpenAI local)",
  "Other OpenAI-compatible server (Mistral, Groq, OpenRouter…)":
    "Autre serveur compatible OpenAI (Mistral, Groq, OpenRouter…)",
  "Embedded engine (llama.cpp, GGUF file)":
    "Moteur embarqué (llama.cpp, fichier GGUF)",
  "5.2 GB · safest, ~17 tok/s": "5,2 Go · le plus sûr, ~17 tok/s",
  "2.5 GB · recommended for a light kit, ~30 tok/s":
    "2,5 Go · recommandé pour un kit léger, ~30 tok/s",
  "1.4 GB · very fast (~60 tok/s), may drift to English":
    "1,4 Go · très rapide (~60 tok/s), peut glisser vers l’anglais",
  "I do not have that information.": "Je n’ai pas cette information.",
  "Add a folder": "Ajouter un dossier",
  "Feed Langolier": "Nourrir Langolier",
  "Remove a folder": "Retirer un dossier",
  "Remove a source": "Retirer une source",
  "Connecting to the engine": "Connexion au moteur",
  "Check the citations: some references are missing or invalid.":
    "Vérifiez les citations : certaines références sont absentes ou invalides.",
  "Export saved.": "Export enregistré.",
  "Delete this conversation": "Supprimer cette conversation",
  "Engine connected": "Moteur connecté",
  "Engine needs setup": "Moteur à configurer",
  "Interface preview": "Aperçu de l’interface",
  "Your machine, your AI": "Votre machine, votre IA",
  "Show or hide the menu": "Afficher ou masquer le menu",
  "Add sources": "Ajouter des sources",
  "Helpful answer": "Réponse utile",
  "Answer needs work": "Réponse à améliorer",
  "Your question": "Votre question",
  "Ask your memory a question…": "Posez une question à votre mémoire…",
  "Your next idea starts with a question…":
    "Votre prochaine idée commence par une question…",
  "Add a source": "Ajouter une source",
  "Conversation mode": "Mode de conversation",
  "Rewrites the question using the history and merges two searches. Costs one extra model call.":
    "Reformule la question avec l’historique et fusionne deux recherches. Ajoute un appel au modèle.",
  "Assistant profile": "Profil d’assistant",
  "Profile used for new conversations":
    "Profil utilisé pour les nouvelles conversations",
  "Stop generating": "Arrêter la génération",
  "YOUR KNOWLEDGE, NO SILOS": "VOTRE SAVOIR, SANS SILOS",
  "The raw material.": "La matière première.",
  "Every source becomes a starting point. Your memory grows with you.":
    "Chaque source devient un point de départ. Votre mémoire grandit avec vous.",
  Send: "Envoyer",
  "The website": "Le site",
  "The source code": "Le code source",
  "Support the project": "Soutenir le projet",
  "This model is not downloaded yet, so nothing can be vectorised. Word search keeps working meanwhile.":
    "Ce modèle n'est pas encore téléchargé : rien ne peut être vectorisé. La recherche par mots continue de fonctionner.",
  "Install it": "L'installer",
  "Built-in embedding engine": "Moteur d'embeddings intégré",
  "The GGUF runs inside Langolier, with nothing to start beside it. Uncheck to use a server of your own.":
    "Le GGUF tourne dans Langolier, sans rien à lancer à côté. Décochez pour utiliser votre propre serveur.",
  "Embedding server address": "Adresse du serveur d'embeddings",
  "An Ollama server, or anything speaking the same API. It has to be running when Langolier searches.":
    "Un serveur Ollama, ou tout ce qui parle la même API. Il doit tourner quand Langolier cherche.",
  "In the background": "En arrière-plan",
  Processing: "Traitement",
  "Accept machine-made subtitles for videos":
    "Accepter les sous-titres générés automatiquement",
  "A video whose author wrote subtitles is read in seconds instead of being transcribed. Machine-made ones are faster too, but carry no punctuation, which cuts poorly into passages.":
    "Une vidéo dont l'auteur a écrit les sous-titres est lue en quelques secondes au lieu d'être transcrite. Les sous-titres automatiques sont rapides aussi, mais sans ponctuation, ce qui se découpe mal en passages.",
  "Looking for subtitles · yt-dlp": "Recherche de sous-titres · yt-dlp",
  "The subtitles held no usable text.":
    "Les sous-titres ne contenaient aucun texte exploitable.",
  "How sources become passages and vectors":
    "Comment les sources deviennent des passages et des vecteurs",
  "Scan every": "Scruter toutes les",
  "Applies to every watch that does not set its own interval.":
    "S'applique à chaque vigie qui ne fixe pas son propre intervalle.",
  "Where Langolier waits when you are not using it":
    "Là où Langolier attend quand vous ne vous en servez pas",
  "Local API": "API locale",
  "Closing the window leaves Langolier in the menu bar, out of the Dock: the interface is released and about 240 MB with it. Click the icon to ask a question, right click for the menu. The local API keeps answering.":
    "Fermer la fenêtre laisse Langolier dans la barre des menus, hors du Dock : l'interface est libérée, et environ 240 Mo avec elle. Cliquez l'icône pour poser une question, clic droit pour le menu. L'API locale continue de répondre.",
  "Keep watching folders in the menu bar":
    "Continuer à surveiller les dossiers dans la barre des menus",
  "Off: a watched folder filled while Langolier sits in the menu bar is only picked up when you open the window again.":
    "Désactivé : un dossier surveillé qui se remplit pendant que Langolier est dans la barre des menus n'est repris qu'à la réouverture de la fenêtre.",
  "Keep processing sources in the menu bar":
    "Continuer à traiter les sources dans la barre des menus",
  "Off: queued sources wait. OCR, transcription and vectors are what heat the machine.":
    "Désactivé : les sources en file attendent. L'OCR, la transcription et les vecteurs sont ce qui chauffe la machine.",
  "Let Shortcuts, Siri and local apps query your Langolier memory.":
    "Autorisez Raccourcis, Siri et les applications locales à interroger votre mémoire Langolier.",
  "Answer local requests": "Répondre aux requêtes locales",
  "Loopback only, never the network. Answers come from the same engine as the window, one question at a time.":
    "Boucle locale uniquement, jamais le réseau. Les réponses viennent du même moteur que la fenêtre, une question à la fois.",
  Port: "Port",
  "Access token": "Jeton d'accès",
  "Sent as Authorization: Bearer. Without it the API answers nothing.":
    "Envoyé dans Authorization: Bearer. Sans lui, l'API ne répond rien.",
  Hide: "Masquer",
  Show: "Afficher",
  "Copy the token": "Copier le jeton",
  Regenerate: "Régénérer",
  "Token copied.": "Jeton copié.",
  "New token. Save, then update your shortcut.":
    "Nouveau jeton. Enregistrez, puis mettez à jour votre raccourci.",
  "Copy the address": "Copier l'adresse",
  "Address copied.": "Adresse copiée.",
  "Test the API": "Tester l'API",
  "Testing…": "Test en cours…",
  "The API answers in {n} ms.": "L'API répond en {n} ms.",
  "The API answered an error.": "L'API a renvoyé une erreur.",
  "The local API is switched off.": "L'API locale est désactivée.",
  "The local API needs a token before it answers.":
    "L'API locale a besoin d'un jeton avant de répondre.",
  "The local API port must be between 1024 and 65535.":
    "Le port de l'API locale doit être compris entre 1024 et 65535.",
  "Wrong or missing token.": "Jeton absent ou incorrect.",
  "Queue up to date": "File à jour",
  "Raw search thresholds, before the judge: minimum cosine":
    "Seuils de recherche bruts, avant le juge : cosinus minimal",
  "Candidates per search arm: {n}": "Candidats par voie de recherche : {n}",
  "Passages each arm brings back before lexical and dense results are merged. Wider catches more, and costs more to rerank.":
    "Passages que chaque voie rapporte avant la fusion des résultats lexicaux et sémantiques. Plus large attrape davantage, et coûte plus cher à reclasser.",
  "Passages per source: {n}": "Passages par source : {n}",
  "Passages per source: no cap": "Passages par source : sans limite",
  "Keeps one talkative source from filling the whole answer. Zero lifts the cap.":
    "Empêche une source bavarde de remplir toute la réponse. Zéro retire la limite.",
  "Passage length: {n} characters": "Longueur d'un passage : {n} caractères",
  "Applies to sources indexed from now on. Existing passages keep the length they were cut with, until a reindex.":
    "S'applique aux sources indexées à partir de maintenant. Les passages existants gardent la longueur avec laquelle ils ont été découpés, jusqu'à une réindexation.",
  "Overlap between passages: {n} characters":
    "Recouvrement entre passages : {n} caractères",
  Queued: "En file",
  Preparing: "Préparation",
  "Needs checking": "À vérifier",
  Duplicate: "Doublon",
  "Remove the duplicates": "Retirer les doublons",
  "Processing ({n})": "En traitement ({n})",
  "Failed ({n})": "En erreur ({n})",
  "unknown reason": "raison inconnue",
  "…and {n} more.": "…et {n} autres.",
  "Remove the {n} duplicate(s) from the index? The source already holding those bytes stays, and the original files are untouched.":
    "Retirer {n} doublon(s) de l'index ? La source qui porte déjà ce contenu reste en place, et les fichiers d'origine ne sont pas touchés.",
  "Duplicate of {name}": "Doublon de {name}",
  "{n} duplicate(s)": "{n} doublon(s)",
  "Remove {n} duplicate(s)": "Retirer {n} doublon(s)",
  "Hybrid index": "Index hybride",
  "Lexical index": "Index lexical",
  "Reading and SHA-256 fingerprint": "Lecture et empreinte SHA-256",
  "Audio extraction · FFmpeg": "Extraction audio · FFmpeg",
  "Text extraction · pdftotext": "Extraction du texte · pdftotext",
  "{n} queued or running": "{n} en attente ou en cours",
  "{n} to check": "{n} à vérifier",
  "Process {n} again": "Relancer {n} source(s)",
  "Process again": "Relancer le traitement",
  "Process the {n} source(s) needing a check again? Processing runs in the background.":
    "Relancer le traitement des {n} source(s) à vérifier ? Le traitement se poursuit en arrière-plan.",
  "{n} source(s) queued again.": "{n} source(s) remises en file.",
  "{n} source(s) queued again; {busy} still processing.":
    "{n} source(s) remises en file ; {busy} encore en traitement.",
  "Show every source": "Afficher toutes les sources",
  "Search inside passages": "Recherche dans les passages",
  "Find a source…": "Retrouver une source…",
  "Filter sources": "Filtrer les sources",
  Media: "Médias",
  "A whole memory to build.": "Toute une mémoire à construire.",
  "Drop a folder, paste text or add a video. Processing continues in the background.":
    "Glissez un dossier, collez du texte ou ajoutez une vidéo. Le traitement se poursuit en arrière-plan.",
  Reindex: "Réindexer",
  "Remove from the index": "Retirer de l’index",
  "No matching source": "Aucune source correspondante",
  "Try another name or another filter.":
    "Essayez un autre nom ou un autre filtre.",
  "Settings saved.": "Configuration enregistrée.",
  "Dismiss error": "Fermer l’erreur",
  "YOUR SPACE": "VOTRE ESPACE",
  "New conversation": "Nouvelle conversation",
  "Your next ideas": "Vos prochaines idées",
  "Personal space": "Espace personnel",
  "Your data stays here": "Vos données restent ici",
  "Make sense of": "Donnez du sens à",
  "Your documents, your code, your videos, your voice notes.":
    "Vos documents, votre code, vos vidéos.",
  "One place to connect them. An AI to explore them.":
    "Un seul endroit pour les relier. Une IA pour les explorer.",
  "FEED YOUR CURIOSITY": "NOURRISSEZ VOTRE CURIOSITÉ",
  "Drag. Drop. Connect.": "Glissez. Déposez. Connectez.",
  "A whole folder, a video or a passing thought.":
    "Un dossier entier, une vidéo ou une simple idée.",
  "Langolier turns it into a memory you can explore.":
    "Langolier en fait une mémoire à explorer.",
  VIDEO: "VIDÉO",
  "sources absorbed": "sources assimilées",
  "passages in memory": "passages en mémoire",
  "languages detected": "langues détectées",
  "Your knowledge.": "Votre savoir.",
  "On your machine.": "Sur votre machine.",
  "THE THREAD OF YOUR THINKING": "LE FIL DE VOTRE RÉFLEXION",
  "[External image not loaded]": "[Image externe non chargée]",
  "Check the sources for anything that matters.":
    "Vérifiez les sources pour les points importants.",
  "Hybrid memory": "Mémoire hybride",
  "Semantic search": "Recherche sémantique",
  "Lexical search": "Recherche lexicale",
  "Free conversation": "Conversation libre",
  "Deep search": "Recherche approfondie",
  "Local inference · Traceable sources": "Inférence locale · Sources traçables",
  "Enter to send": "Entrée pour envoyer",
  List: "Liste",
  Tree: "Arbre",
  Search: "Recherche",
  Name: "Nom",
  "Add your first source": "Ajouter votre première source",
  LANGUAGE: "LANGUE",
  PROCESSING: "TRAITEMENT",
  "See details": "Voir le détail",
  "Something to check": "Un point à vérifier",
  "Shall we turn this into something?": "On en fait une nouvelle idée ?",
  "Drop your files to add them to your memory.":
    "Déposez vos fichiers pour les ajouter à votre mémoire.",
  "API key": "Clé d’API",
  "Saved key": "Clé enregistrée",
  "Pick a saved entry…": "Choisir une entrée enregistrée…",
  "Vault empty": "Coffre vide",
  "Remove from the vault": "Retirer du coffre",
  "Paste a key…": "Collez une clé…",
  "Key name": "Nom de la clé",
  "The vault saves you retyping keys. They sit in Langolier's local database in clear text, like the rest of your settings. Protect your session.":
    "Le coffre évite de retaper vos clés. Elles restent dans la base locale de Langolier, en clair comme le reste de vos réglages — protégez votre session.",
  "Save as…": "Enregistrer sous…",
  "A little more material.": "Un peu plus de matière.",
  "Pasted text": "Texte collé",
  "Video link": "Lien vidéo",
  "e.g. Lecture notes on transformers":
    "Ex. Notes du cours sur les transformers",
  "Your text": "Votre texte",
  "Paste an article, your notes, a code snippet…":
    "Collez un article, vos notes, un extrait de code…",
  "Add what you want to find again, understand and connect.":
    "Ajoutez ce que vous voulez retrouver, comprendre et relier.",
  "Drop your files here": "Glissez vos fichiers ici",
  "PDF, DOCX, MD, PY, IPYNB, MP4, MP3, SRT and more":
    "PDF, MD, PY, IPYNB, MP4, MP3, SRT et plus encore",
  "Import a whole folder": "Importer un dossier entier",
  "Source title": "Titre de la source",
  "Video address": "Adresse de la vidéo",
  Download: "Téléchargement",
  Indexing: "Indexation",
  "Videos & transcripts": "Vidéos & transcriptions",
  "PDF documents": "Documents PDF",
  Data: "Données",
  "View indexed content": "Voir le contenu indexé",
  "Search a word, an exact phrase, or an idea…":
    "Chercher un mot, une phrase exacte, ou une idée…",
  "Search in memory": "Recherche dans la mémoire",
  "Text to replace in the results": "Texte à remplacer dans les résultats",
  "Replace with": "Remplacer par",
  "Open the source": "Ouvrir la source",
  "Edit this passage": "Modifier ce passage",
  "Nothing found": "Rien trouvé",
  "Try another mode: Words ignores order, Meaning looks for the idea.":
    "Essayez un autre mode : « Mots » tolère l’ordre, « Sens » cherche l’idée.",
  "The originals must stay reachable where they are.":
    "Les originaux doivent rester accessibles à leur emplacement.",
  "Reindex sources": "Réindexer des sources",
  "By type": "Par type",
  "Select all": "Tout cocher",
  "Clear all": "Tout décocher",
  Cancel: "Annuler",
  "No correction proposed for this passage.":
    "Aucune correction proposée pour ce passage.",
  "Fix the source with AI": "Corriger la source par l’IA",
  "Fixing…": "Correction en cours…",
  "Delete a passage": "Supprimer un passage",
  "Text to highlight / replace": "Texte à surligner / remplacer",
  "Previous match (Shift+Enter)": "Occurrence précédente (Maj+Entrée)",
  "Next match (Enter)": "Occurrence suivante (Entrée)",
  "In this source": "Dans cette source",
  "Across the whole memory": "Dans toute la mémoire",
  "Transcription and OCR errors only: misheard words, technical terms, punctuation. Nothing is rewritten.":
    "Erreurs de transcription ou d’OCR uniquement : mots mal entendus, termes techniques, ponctuation. Rien n’est reformulé.",
  "Propose an AI correction": "Proposer une correction par l’IA",
  "Fix this passage": "Corriger ce passage",
  "Delete this passage": "Supprimer ce passage",
  "No indexed text yet": "Pas encore de texte indexé",
  "Processing is queued or needs attention. Check its status under Sources.":
    "Le traitement est en attente ou nécessite une intervention. Consultez son statut dans Sources.",
  Count: "Compter",
  "Preview limited to the first 500 passages.":
    "Aperçu limité aux 500 premiers passages.",
  "Unreadable image": "Image illisible",
  "Checking…": "Vérification…",
  "Delete the assistant": "Supprimer l’assistant",
  "Folder to create the chatbot in": "Dossier où créer le chatbot",
  "Folder to write the .langolier file to":
    "Dossier où écrire le fichier .langolier",
  "ONE VOICE PER MISSION": "UNE VOIX PAR MISSION",
  "The assistants.": "Les assistants.",
  "Each profile has its own name, mission, knowledge scope and settings. Export it as a standalone chatbot when it is ready.":
    "Chaque profil a son nom, sa mission, son périmètre de connaissances et ses réglages. Exportez-le en chatbot autonome quand il est prêt.",
  "The whole memory": "Toute la mémoire",
  "Active profile": "Profil actif",
  "Use this profile in the conversation":
    "Utiliser ce profil dans la conversation",
  "No profile": "Aucun profil",
  "Create an assistant: a mission, a scope, a tone.":
    "Créez un assistant : une mission, un périmètre, un ton.",
  "New assistant": "Nouvel assistant",
  "Choose an image": "Choisir une image",
  "Nanny, Lawyer, Archivist…": "Nounou, Juriste, Archiviste…",
  "Hello! Ask me anything about…": "Bonjour ! Posez-moi vos questions sur…",
  "Who this assistant is, who it talks to, what it must and must not do, its tone.":
    "Qui est cet assistant, à qui il parle, ce qu’il doit faire et ne pas faire, son ton.",
  "Filter sources…": "Filtrer les sources…",
  "(filtered)": "(filtré)",
  "API key for this profile": "Clé d’API de ce profil",
  "Bot token (created with @BotFather)": "Jeton du bot (créé avec @BotFather)",
  "The profile's chat model is swapped for qwen3:4b-instruct (pulled through Ollama if missing): same behaviour on strict grounding in our tests, twice as fast, half the size. The profile's embedder is bundled and the knowledge re-vectorised.":
    "Le modèle de conversation du profil est remplacé par qwen3:4b-instruct (téléchargé via Ollama s’il manque) : même comportement sur l’ancrage strict dans nos tests, deux fois plus rapide, moitié moins lourd. L’embeddeur du profil est joint et les connaissances revectorisées.",
  "The chat model is copied from Ollama (several GB); EmbeddingGemma is downloaded once then bundled (330 MB). Knowledge is re-vectorised by the embedded engine. With a cloud profile, only the embedder is bundled.":
    "Le modèle de conversation est copié depuis Ollama (plusieurs Go) ; EmbeddingGemma est téléchargé une fois puis joint (330 Mo). Les connaissances sont revectorisées par le moteur embarqué. Avec un profil cloud, seul l’embeddeur est joint.",
  "Light kit. The manual lists the expected models; the launchers pull them through Ollama and report what is missing on the page.":
    "Kit léger. Le mode d’emploi liste les modèles attendus ; les lanceurs les téléchargent via Ollama et signalent ce qui manque dans la page.",
  "The knowledge and profile file, to create or update an already deployed chatbot":
    "Le fichier de connaissances et de profil, pour créer ou mettre à jour un chatbot déjà déployé",
  "A ready-to-run folder: the .langolier, a window app, a web app, the server scripts and the manual":
    "Un dossier prêt à l’emploi : le .langolier, une app fenêtre, une app web, les scripts serveur et le mode d’emploi",
  "No profile · the whole memory": "Sans profil · toute la mémoire",
  "Identity and mission": "Identité et mission",
  "Welcome message": "Message d’accueil",
  "Remove the image": "Retirer l’image",
  "Show cited sources in the exported chatbot":
    "Afficher les sources citées dans le chatbot exporté",
  "Hide source file names": "Masquer les noms de fichiers des sources",
  "Cap per conversation (tokens, 0 = unlimited)":
    "Plafond par conversation (tokens, 0 = illimité)",
  "Cap per day, all conversations (tokens, 0 = unlimited)":
    "Plafond par jour, toutes conversations (tokens, 0 = illimité)",
  "Chat page theme": "Thème de la page de conversation",
  "Knowledge scope": "Périmètre de connaissances",
  "What the assistant is allowed to read":
    "Ce que l’assistant a le droit de lire",
  Selection: "Sélection",
  "Own settings": "Réglages propres",
  "Empty = Langolier's global setting": "Vide = réglage global de Langolier",
  Provider: "Fournisseur",
  Model: "Modèle",
  Temperature: "Température",
  "Passages retrieved": "Passages récupérés",
  "Abstention sentence for this profile": "Phrase d’abstention de ce profil",
  "Answer only from the sources": "Répondre uniquement depuis les sources",
  "Relevance judge": "Juge de pertinence",
  "The exported chatbot also answers on Telegram":
    "Le chatbot exporté répond aussi dans Telegram",
  "Test the token": "Tester le jeton",
  "Who may talk to it (empty = everyone)":
    "Qui peut lui parler (vide = tout le monde)",
  "Engine for the exported kit": "Moteur du kit exporté",
  "Follow the profile (cloud, or Ollama on the target machine)":
    "Selon le profil (cloud, ou Ollama sur la machine cible)",
  "Complete: llama.cpp + bundled GGUF models, no prerequisites":
    "Complet : llama.cpp + modèles GGUF joints, aucun prérequis",
  Close: "Fermer",
  "Export .langolier": "Exporter .langolier",
  "Export the chatbot": "Exporter le chatbot",
  "Save the profile first to be able to export it.":
    "Enregistrez d’abord le profil pour pouvoir l’exporter.",
  "The embedding model changed: existing vectors are no longer comparable. Pick the sources to reindex now? (Word search keeps working meanwhile.)":
    "Le modèle d’embeddings a changé : les vecteurs existants ne sont plus comparables. Choisir les sources à réindexer maintenant ? (La recherche par mots continue de fonctionner pendant ce temps.)",
  "Reindexing needed": "Réindexation nécessaire",
  "Connecting to the Ollama registry": "Connexion au registre Ollama",
  "Model installed.": "Modèle installé.",
  "Whisper ggml model": "Modèle Whisper ggml",
  "THE POWER IS YOURS": "LA PUISSANCE VOUS APPARTIENT",
  "Choose your engine.": "Choisissez votre moteur.",
  "Swap models, keep your memory. Everything runs on your machine.":
    "Changez de modèle, gardez votre mémoire. Tout s’exécute sur votre machine.",
  "Your engine is answering.": "Votre moteur répond présent.",
  "Let's connect your local engine.": "Connectons votre moteur local.",
  "Start Ollama or a local OpenAI-compatible server.":
    "Démarrez Ollama ou un serveur local compatible OpenAI.",
  "GGUF file for the chat model": "Fichier GGUF du modèle de conversation",
  "Chat model": "Modèle de conversation",
  "Press the combination…": "Appuyez sur la combinaison…",
  "Model to download": "Modèle à télécharger",
  "Downloading…": "Téléchargement…",
  "Install the model": "Installer le modèle",
  "Database backed up. Original media must be backed up separately.":
    "Base sauvegardée. Les médias originaux sont à sauvegarder séparément.",
  "Local application directory": "Répertoire local de l’application",
  Check: "Vérifier",
  "The brain": "Le cerveau",
  "Generation and conversation": "Génération et conversation",
  "Server address": "Adresse du serveur",
  "Context window": "Fenêtre de contexte",
  "The memory": "La mémoire",
  "Representation and search": "Représentation et recherche",
  'Ollama embedding server (or "embedded" for a local GGUF)':
    "Serveur d’embeddings Ollama (ou « embedded » pour un GGUF local)",
  "GGUF file for the embedding model": "Fichier GGUF du modèle d’embeddings",
  "Embedding model": "Modèle d’embeddings",
  "The discipline": "La rigueur",
  "What the assistant is allowed to answer":
    "Ce que l’assistant a le droit de répondre",
  "Abstention sentence": "Phrase d’abstention",
  "Relevance judge (reranker)": "Juge de pertinence (reranker)",
  "Judge model (empty = chat model)":
    "Modèle juge (vide = modèle de conversation)",
  "Verify the answer after writing": "Vérifier la réponse après rédaction",
  "Minimum BM25 score (0 = off)": "Score BM25 minimal (0 = désactivé)",
  "The palette": "La palette",
  "Ask a question from anywhere": "Poser une question depuis n’importe où",
  "Global shortcut": "Raccourci global",
  "Try the palette": "Essayer la palette",
  "The ears": "Les oreilles",
  "Local multilingual transcription through whisper.cpp":
    "Transcription multilingue locale via whisper.cpp",
  "Whisper model file": "Fichier du modèle Whisper",
  Choose: "Choisir",
  "A new model, in one click": "Un nouveau modèle, en un clic",
  "Downloaded by the Ollama server configured for embeddings":
    "Téléchargement par le serveur Ollama configuré pour les embeddings",
  "Maintain your memory": "Entretenir votre mémoire",
  "Rebuild the index or take a consistent backup of the database.":
    "Reconstruire l’index ou créer une sauvegarde cohérente de la base.",
  "Reindex sources…": "Réindexer des sources…",
  "Back up the database": "Sauvegarder la base",
  "Scanned pages": "Les pages scannées",
  "Your memory lives here": "Votre mémoire est ici",
  "INTUITION, TESTED AGAINST FACTS": "L’INTUITION, À L’ÉPREUVE DES FAITS",
  "The laboratory.": "Le laboratoire.",
  "Inspect retrieval. Compare strategies. Build your reference set.":
    "Inspectez la recherche. Comparez les stratégies. Construisez votre jeu de référence.",
  "Search question": "Question de recherche",
  "What are you looking for?": "Quelle information cherchez-vous ?",
  "Reference question": "Question de référence",
  "e.g. How do I limit overfitting?":
    "Ex. Comment limiter le surapprentissage ?",
  "Expected source": "Source attendue",
  "Source removed": "Source retirée",
  "Delete this case": "Supprimer ce cas",
  "Retrieval microscope": "Microscope de recherche",
  Hybrid: "Hybride",
  Semantic: "Sémantique",
  "Benchmark your corpus": "Benchmark de votre corpus",
  Add: "Ajouter",
  "Total time": "Durée totale",
  "Errors / fallbacks": "Erreurs / replis",
  "FROM CONVERSATION TO DATASET": "DE LA CONVERSATION AU DATASET",
  "Learn from what works.": "Apprenez de ce qui fonctionne.",
  "EXPORT READY · TRAINING OUTSIDE THE APP":
    "EXPORT PRÊT · ENTRAÎNEMENT HORS INTERFACE",
  "Export the dataset": "Exporter le dataset",
  "COMPLETED QUERIES": "REQUÊTES TERMINÉES",
  "Every conversation attempt": "Toutes les tentatives de conversation",
  "TECHNICAL SUCCESS": "SUCCÈS TECHNIQUE",
  "Completed generations / attempts": "Générations abouties / tentatives",
  "MODEL SPEED": "VITESSE DU MODÈLE",
  "Tokens per second, measured at the engine":
    "Tokens / seconde, mesure du moteur",
  "POSITIVE FEEDBACK": "RETOURS POSITIFS",
  "MEASURE TO UNDERSTAND": "MESURER POUR COMPRENDRE",
  "Under the hood.": "Sous le capot.",
  "What works, what drags, what deserves your attention.":
    "Ce qui fonctionne, ce qui ralentit, ce qui mérite votre attention.",
  "Open the native app to measure": "Ouvrez l’application native pour mesurer",
  Evaluation: "Évaluation",
  "Document pipeline": "Pipeline documentaire",
  "Export the metrics": "Exporter les métriques",
  "The rhythm of your exchanges": "Le rythme de vos échanges",
  "Total latency of the last 24 attempts":
    "Latence totale des 24 dernières tentatives",
  "REAL MEASUREMENTS": "MESURES RÉELLES",
  Oldest: "Plus ancien",
  Now: "Maintenant",
  "The first exchange will draw the curve.":
    "Le premier échange dessinera la courbe.",
  "No simulated data.": "Aucune donnée simulée.",
  "The local footprint": "L’empreinte locale",
  "Your machine's resources": "Ressources de votre machine",
  "system memory used": "mémoire système utilisée",
  "Indexed text": "Texte indexé",
  "Tokens generated": "Tokens générés",
  "Average latency": "Latence moyenne",
  "Activity log": "Journal d’activité",
  "Ingestion, conversations and evaluations":
    "Ingestion, conversations et évaluations",
  "LAST 100 EVENTS": "100 DERNIERS ÉVÉNEMENTS",
  "Your activity will show up here.": "Votre activité apparaîtra ici.",
  "Ask your memory…": "Demandez à votre mémoire…",
  "Global setting": "Réglage global",
  "Folder to watch": "Dossier à surveiller",
  "Remove the watch": "Retirer la vigie",
  "Forget the watch and its knowledge": "Oublier la vigie et son savoir",
  "YOUR MEMORY KEEPS WATCH": "VOTRE MÉMOIRE VEILLE",
  "The watches.": "Les vigies.",
  "Folders watched continuously, local or on a mounted NAS. Whatever lands there enters your memory on its own.":
    "Des dossiers surveillés en continu — locaux ou sur un NAS monté. Ce qui y arrive entre dans votre mémoire tout seul.",
  "Pause processing of queued files without losing anything":
    "Suspendre le traitement des fichiers déjà en file, sans rien perdre",
  "Resume processing": "Reprendre le traitement",
  "Pause processing": "Suspendre le traitement",
  "Disable the watch": "Désactiver la vigie",
  "Enable the watch": "Activer la vigie",
  "Include subfolders": "Inclure les sous-dossiers",
  "See this watch's files": "Voir les fichiers de cette vigie",
  "Scan now": "Scruter maintenant",
  "Remove the watch (keep the knowledge)":
    "Retirer la vigie (garder le savoir)",
  "Remove the watch and forget its sources":
    "Retirer la vigie et oublier ses sources",
  "No watch yet.": "Aucune vigie pour l’instant.",
  "Add a folder: every supported file that appears there gets indexed, and changed files are reprocessed.":
    "Ajoutez un dossier : tout fichier pris en charge qui y apparaît sera indexé, et ceux qui changent seront retraités.",
  Retry: "Réessayer",
  "No tracked file": "Aucun fichier suivi",
  "This watch has found nothing yet, or its sources were detached.":
    "Cette vigie n’a encore rien trouvé, ou ses sources ont été détachées.",
  Sync: "Suivre",
  Hoover: "Aspirer",
  Subfolders: "Sous-dossiers",
  "Retry everything": "Tout réessayer",
  "No indexed source yet.": "Aucune source indexée pour l’instant.",
  "{n} source(s) added": "{n} source(s) ajoutée(s)",
  " · {n} skipped": " · {n} ignorée(s)",
  'Remove the {n} sources under "{label}" from the index? The original files are untouched.':
    "Retirer les {n} sources de « {label} » de l’index ? Les fichiers d’origine ne sont pas touchés.",
  'Remove "{label}" from the index? The original file is untouched.':
    "Retirer « {label} » de l’index ? Le fichier d’origine n’est pas touché.",
  "{n} source(s) removed; {busy} still processing, try again after.":
    "{n} source(s) retirée(s) ; {busy} en cours de traitement, réessayez ensuite.",
  'Delete "{title}"': "Supprimer « {title} »",
  "{size} of unified memory / RAM": "{size} de mémoire unifiée / RAM",
  'Key "{label}" is in place.': "Clé « {label} » en place.",
  'Saved as "{label}" ({hint}).': "Enregistrée sous « {label} » ({hint}).",
  'Remove "{label}" from the vault?': "Retirer « {label} » du coffre ?",
  '"{label}" removed from the vault.': "« {label} » retirée du coffre.",
  "Select everything in {label}": "Tout sélectionner dans {label}",
  'Remove the {n} sources under "{label}"':
    "Retirer les {n} sources de « {label} »",
  "{n} occurrence(s) replaced across {chunks} passage(s).":
    "{n} occurrence(s) remplacée(s) dans {chunks} passage(s).",
  "{n} source(s) put back in the queue.":
    "{n} source(s) remise(s) dans la file.",
  " {n} still processing, try again after.":
    " {n} en cours de traitement, réessayez ensuite.",
  " The originals must stay reachable where they are.":
    " Les originaux doivent rester accessibles à leur emplacement.",
  "Fixing {done}/{total} · {changed} passage(s) changed":
    "Correction {done}/{total} · {changed} passage(s) modifié(s)",
  "Have the model reread the {n} passage(s) of this source? Only recognition errors are fixed; changed passages are reindexed. The original on disk stays intact.":
    "Faire relire les {n} passage(s) de cette source par le modèle ? Seules les erreurs de reconnaissance sont corrigées ; les passages modifiés sont réindexés. L’original sur disque reste intact.",
  "{changed} passage(s) fixed out of {chunks}":
    "{changed} passage(s) corrigé(s) sur {chunks}",
  ", {n} proposal(s) rejected": ", {n} proposition(s) rejetée(s)",
  " (interrupted)": " (interrompu)",
  "Delete this passage ({locator}) from memory? The original on disk is not modified.":
    "Supprimer ce passage ({locator}) de la mémoire ? L’original sur disque n’est pas modifié.",
  "Bot recognised: @{username} ({name}).":
    "Bot reconnu : @{username} ({name}).",
  '"{name}" saved.': "« {name} » enregistré.",
  'Delete the profile "{name}"? Conversations and memory stay.':
    "Supprimer le profil « {name} » ? Les conversations et la mémoire restent.",
  "Chatbot exported to {path}: {launchers} + .langolier ({size}).":
    "Chatbot exporté dans {path} : {launchers} + .langolier ({size}).",
  "File written: {path} ({size}).": "Fichier écrit : {path} ({size}).",
  "{n} model(s) available": "{n} modèle(s) disponible(s)",
  "{n} answer(s) rated by hand": "{n} réponse(s) évaluée(s) manuellement",
  'Remove the watch "{path}"?\n\nOK: drop the watch and keep the {n} source(s) already ingested.\nCancel: do nothing.':
    "Retirer la vigie « {path} » ?\n\nOK : retirer la vigie et garder les {n} source(s) déjà ingérées.\nAnnuler : ne rien faire.",
  "Remove the watch AND forget its {n} source(s) (passages and vectors dropped from memory)? Files on disk are untouched.":
    "Retirer la vigie ET oublier ses {n} source(s) (passages et vecteurs supprimés de la mémoire) ? Les fichiers sur disque ne sont pas touchés.",
  "Global setting ({n} s)": "Réglage global ({n} s)",
  "{pct}% indexed": "{pct} % indexé",
  "Visual preview. Sources, models and metrics become active in the native app.":
    "Aperçu visuel. Les sources, modèles et métriques deviennent actifs dans l’application native.",
  "ONE MEMORY. A THOUSAND CONNECTIONS.": "UNE MÉMOIRE. MILLE CONNEXIONS.",
  "everything you know.": "tout ce que vous savez.",
  "Add a video link": "Ajouter un lien vidéo",
  "⇧ Enter for a new line": "⇧ Entrée pour une nouvelle ligne",
  "Originals stay where they are. Removing a source drops its passages and vectors from the index. Older messages and their citations stay in the history.":
    "Les originaux restent à leur emplacement. Retirer une source supprime ses passages et ses vecteurs de l’index. Les anciens messages et leurs citations restent dans l’historique.",
  "Subfolders included. .git, node_modules, target and Python environments are skipped.":
    "Sous-dossiers inclus. .git, node_modules, target et environnements Python ignorés.",
  characters: "caractères",
  "Add to my memory": "Ajouter à ma mémoire",
  "Language detected automatically. Audio is transcribed with timestamps. Availability depends on the site and on access rights; protected content is not supported.":
    "Langue détectée automatiquement. L’audio est transcrit avec des repères temporels. La disponibilité dépend du site et des droits d’accès, les contenus protégés ne sont pas pris en charge.",
  "Add this video": "Ajouter cette vidéo",
  "Reindexing rereads the original files and recomputes passages and vectors. Required after an embedding model change, pointless otherwise.":
    "La réindexation relit les fichiers d’origine et recalcule passages et vecteurs. Indispensable après un changement de modèle d’embeddings ; inutile autrement.",
  "Unchecked: the chatbot answers without listing passages, useful for a non-technical audience.":
    "Décoché : le chatbot répond sans lister les passages — utile pour un public non technique.",
  'The model and the page only see "Source 1", "Source 2"… Your document names cannot appear in an answer.':
    "Le modèle et la page ne voient que « Source 1 », « Source 2 »… Les noms de vos documents ne peuvent pas apparaître dans une réponse.",
  "Recommended with an API key: the daily cap bounds the bill whatever happens. Estimated at 4 characters per token over exchanged messages; past the cap the chatbot replies with a fixed message without calling the model.":
    "Recommandé avec une clé d’API : le plafond quotidien borne la facture quoi qu’il arrive. Estimation à 4 caractères par token sur les messages échangés ; au-delà, le chatbot répond par un message fixe sans appeler le modèle.",
  "Watches: a checked watch brings everything it indexes, including files yet to come":
    "Vigies — une vigie cochée apporte tout ce qu’elle indexe, y compris les fichiers à venir",
  "Checking a folder takes everything inside it, subfolders included. Unchecking a subfolder or a file afterwards narrows the selection.":
    "Cocher un dossier prend tout son contenu, sous-dossiers compris ; décocher ensuite un sous-dossier ou un fichier affine la sélection.",
  "The bridge polls the Telegram servers: no port to open, the web launcher or":
    "Le pont fonctionne par interrogation des serveurs Telegram : aucun port à ouvrir, il suffit que le lanceur web ou",
  "just has to be running. One Telegram chat = one conversation;":
    "tourne. Un chat Telegram = une conversation ;",
  "opens another one. Caps and source privacy apply. Messages travel through Telegram, so keep it to content that may go there.":
    "en ouvre une autre. Les plafonds et la confidentialité des sources s’appliquent. Les messages transitent par Telegram : à réserver à des contenus qui peuvent y circuler.",
  "Complete light: same with qwen3:4b-instruct (≈ 2.9 GB total)":
    "Complet léger : idem avec qwen3:4b-instruct (≈ 2,9 Go au total)",
  "The exported chatbot is an executable for this system (":
    "Le chatbot exporté est un exécutable pour ce système (",
  ") carrying the profile, its settings and its knowledge. With a cloud provider it runs anywhere; with Ollama the target machine must have it installed. To update a deployed chatbot, drop a new":
    ") qui embarque le profil, ses réglages et ses connaissances. Avec un fournisseur cloud, il fonctionne partout ; avec Ollama, la machine cible doit l’avoir installé. Pour mettre à jour un chatbot déployé, déposez un nouveau",
  "in its folder.": "dans son dossier.",
  "Pick a profile on the left, or create one. The star marks the profile used in the conversation and the palette.":
    "Choisissez un profil à gauche, ou créez-en un. L’étoile désigne le profil utilisé dans la conversation et la palette.",
  "Your questions and the retrieved passages are sent to this provider. The memory (index, vectors) stays local. The key is stored in the application database.":
    "Vos questions et les passages retrouvés sont envoyés à ce fournisseur. La mémoire (index, vecteurs) reste locale. La clé est stockée dans la base de l’application.",
  "Watch out for the bare": "Attention au",
  "(the Thinking-2507 variant): it always reasons and leaks that reasoning into the answer. Take":
    "nu (variante « Thinking-2507 ») : il raisonne toujours et laisse fuir sa réflexion dans la réponse ; prenez",
  "The starting profile uses a quantised 8B model. With 24 GB, keep the context window reasonable. LM Studio manages its window in the server.":
    "Le profil initial utilise un modèle 8B quantifié. Avec 24 Go, gardez une fenêtre de contexte raisonnable. LM Studio gère sa fenêtre dans le serveur.",
  "Curated list: every model has an official GGUF for the complete export. Changing model invalidates the vectors, and a reindex will be offered.":
    "Liste maîtrisée : chaque modèle a un GGUF officiel pour l’export complet. Changer de modèle invalide les vecteurs : une réindexation vous sera proposée.",
  "Changing the embedding means reindexing the sources. Vectors from different models are never mixed. Current configured index:":
    "Changer l’embedding demande de réindexer les sources. Les vecteurs de modèles différents ne sont jamais mélangés. Index actuel configuré :",
  "BM25 plus vector search, RRF fusion, document diversity and section context. Deep search adds a conversational rewrite.":
    "BM25 + recherche vectorielle, fusion RRF, diversité documentaire et contexte de section. La recherche approfondie ajoute une reformulation conversationnelle.",
  "With no convincing passage, the model replies with the abstention sentence instead of drawing on its own knowledge.":
    "Sans passage probant, le modèle répond par la phrase d’abstention au lieu de puiser dans ses connaissances.",
  "A model rereads the candidates and scores each passage. Below the threshold the assistant abstains before writing anything.":
    "Un modèle relit les candidats et note chaque passage. Sous le seuil, l’assistant s’abstient avant même de rédiger.",
  "A second call checks that every claim is supported by the passages. Safer, but it doubles the response time.":
    "Un second appel contrôle que chaque affirmation est soutenue par les passages. Plus sûr, mais double le temps de réponse.",
  "A combination with at least one modifier (⌘, ⌃, ⌥). The fn key cannot be captured by applications. The shortcut opens a floating bar; Esc or a click outside closes it.":
    "Une combinaison avec au moins un modificateur (⌘, ⌃, ⌥). La touche fn n’est pas capturable par les applications. Le raccourci ouvre une barre flottante ; Échap ou un clic à l’extérieur la ferme.",
  "Run Install-Mac.command or Install-Windows.bat to prepare the engines. The multilingual base model favours speed; small or medium can improve transcription. This version indexes speech, not video images.":
    "Lancez Install-Mac.command ou Install-Windows.bat pour préparer les moteurs. Le modèle multilingue base privilégie la vitesse ; small ou medium peuvent améliorer la transcription. Cette version indexe la parole, pas les images de la vidéo.",
  "An Internet connection is needed to download the weights. Then select the model in its configuration and save.":
    "Une connexion Internet est nécessaire pour télécharger les poids. Sélectionnez ensuite le modèle dans sa configuration, puis enregistrez.",
  "Local optical recognition, switched on automatically page by page.":
    "Reconnaissance optique locale, activée automatiquement page par page.",
  "Pages holding fewer than 30 alphanumeric characters are rendered then read by Tesseract. French and English are installed. Every excerpt keeps its page and the OCR note. Diagrams and formulas may need checking.":
    "Les pages contenant moins de 30 caractères alphanumériques sont rendues puis analysées par Tesseract. Français et anglais installés. Chaque extrait garde sa page et la mention OCR. Les schémas et les formules peuvent nécessiter une vérification.",
  "The passages retrieved before any generation, with their fusion score.":
    "Les passages retrouvés avant toute génération, avec leur score de fusion.",
  "One question, one expected document. Compare BM25, semantic and hybrid.":
    "Une question, un document attendu. Comparez BM25, sémantique et hybride.",
  "Hit@K: expected document present in the K passages. MRR: mean reciprocal rank. These measure retrieval, not the truth of an answer. Results are kept in the log and in the metrics export.":
    "Hit@K : document attendu présent dans les K passages. MRR : rang réciproque moyen. Ces mesures évaluent la recherche, pas la véracité d’une réponse. Résultats conservés dans le journal et l’export de métriques.",
  "Export the answers you approved as JSONL to prepare a LoRA fine-tune. The MLX recipe and the data separation protocol ship with the project.":
    "Exportez les réponses que vous avez validées en JSONL pour préparer un fine-tuning LoRA. La recette MLX et le protocole de séparation des données sont inclus dans le projet.",
  "Technical success does not measure accuracy. Rate retrieval relevance in the Laboratory and check answers against their sources.":
    "Le succès technique ne mesure pas l’exactitude. Évaluez la pertinence de la recherche dans le Laboratoire et vérifiez les réponses contre leurs sources.",
  "Processing paused: watches keep spotting files and the queue grows, but nothing is ingested. Nothing is lost, everything resumes on restart.":
    "Traitement suspendu : les vigies continuent de repérer les fichiers et la file s’allonge, mais rien n’est ingéré. Rien n’est perdu, tout repart à la reprise.",
  "leaves files where they are and reprocesses the ones that change.":
    "laisse les fichiers en place et retraite ceux qui changent.",
  "moves each file into Langolier's data folder then indexes it: the watched folder empties, the original is kept for a future reindex.":
    "déplace chaque fichier dans le dossier de données de Langolier puis l’indexe : le dossier surveillé se vide, l’original est conservé pour une réindexation future.",
  ready: "prêtes",
  "Turning a card off pauses watching without erasing anything. Network folders are polled rather than notified: an unmounted volume is flagged on the card and picked up again as soon as it reappears.":
    "Désactiver une carte suspend la surveillance sans rien effacer. Les dossiers réseau sont scrutés périodiquement plutôt que par notification : un volume démonté est signalé sur la carte et repris dès qu’il réapparaît.",
  Indexed: "Indexées",
  "{n} selected out of {total}": "{n} sélectionnée(s) sur {total}",
  "Reindex {n} source(s)": "Réindexer {n} source(s)",
  " (filtered)": " (filtré)",
  "Maximum answer length: {n} tokens":
    "Longueur maximale de réponse : {n} tokens",
  "Passages retrieved: {n}": "Passages récupérés : {n}",
  Language: "Langue",
  Watches: "Vigies",
  Observatory: "Observatoire",
  Laboratory: "Laboratoire",
  Engines: "Moteurs",
  "start here.": "commencent ici.",
  "Import files": "Importer des fichiers",
  "Paste text": "Coller un texte",
  "Give the assistant a name.": "Donnez un nom à l'assistant.",
  "Mission capped at 6,000 characters, welcome at 600.":
    "Mission limitée à 6 000 caractères, accueil à 600.",
  "The avatar must be an image (PNG or JPEG) under 450 KB.":
    "L'avatar doit être une image (PNG ou JPEG) de moins de 450 Ko.",
  "Invalid provider or model in the profile.":
    "Fournisseur ou modèle invalide dans le profil.",
  "Profile settings out of range.": "Réglages du profil hors limites.",
  "(embedded)": "(embarqué)",
  "Downloading {model} through Ollama": "Téléchargement de {model} via Ollama",
  "Ollama unreachable while downloading {model}: {e}":
    "Ollama injoignable pour télécharger {model} : {e}",
  "Ollama refused to download {model}: {}":
    "Ollama refuse le téléchargement de {model} : {}",
  "Downloading {model}: {e}": "Téléchargement de {model} : {e}",
  'Ollama model "{name}" not found in {}':
    "Modèle Ollama « {name} » introuvable dans {}",
  "Ollama manifest without a model layer":
    "Manifeste Ollama sans couche modèle",
  "Downloading {}: {e}": "Téléchargement de {} : {e}",
  "Download of {} refused: {}": "Téléchargement de {} refusé : {}",
  "Downloading the embedding model": "Téléchargement du modèle d'embeddings",
  "The downloaded file is not a GGUF.":
    "Le fichier téléchargé n'est pas un GGUF.",
  "Copying the chat model": "Copie du modèle de conversation",
  "Copying the embedding model": "Copie du modèle d'embeddings",
  "I do not find that information in the indexed sources.":
    "Je ne trouve pas cette information dans les sources indexées.",
  "GGUF embedding model not found: {}":
    "Modèle d'embeddings GGUF introuvable : {}",
  "Settings out of range": "Paramètres hors limites",
  "Grounding settings out of range": "Réglages d'ancrage hors limites",
  "Source missing or already processing":
    "Source absente ou déjà en traitement",
  "Selection empty or too large.": "Sélection vide ou trop grande.",
  "Wait for the current generation to finish.":
    "Attendez la fin de la génération en cours.",
  "Text saved, lexical index up to date. Vectors not recomputed: {e}":
    "Texte enregistré, index lexical à jour. Vecteurs non recalculés : {e}",
  "The passage must hold between 1 and 20,000 characters.":
    "Le passage doit contenir entre 1 et 20 000 caractères.",
  "A generation is already running": "Une génération est déjà en cours",
  cancelled: "annulée",
  "Selection empty or text out of range.":
    "Sélection vide ou texte hors limites.",
  "The replacement would empty a passage; delete it instead.":
    "Le remplacement viderait un passage ; supprimez-le plutôt.",
  "The text to replace must hold between 1 and 500 characters.":
    "Le texte à remplacer doit contenir entre 1 et 500 caractères.",
  "Image capped at 15 MB.": "Image limitée à 15 Mo.",
  "Give this key a short name.": "Donnez un nom court à cette clé.",
  "This key is not a plausible length.":
    "Cette clé n'a pas une longueur plausible.",
  "Key not found": "Clé introuvable",
  "Select an existing source": "Sélectionnez une source existante",
  "Add at least one reference question.":
    "Ajoutez au moins une question de référence.",
  "No approved answer. Rate some answers positively before exporting.":
    "Aucune réponse validée. Évaluez des réponses positivement avant l'export.",
  "Invalid model name": "Nom de modèle invalide",
  "Download interrupted": "Téléchargement interrompu",
  "Embedded engine unavailable": "Moteur embarqué indisponible",
  "Embedded engine stopped": "Moteur embarqué arrêté",
  "Model not found: {}": "Modèle introuvable : {}",
  "The engine did not answer": "Le moteur n'a pas répondu",
  "Embedding model not found: {}": "Modèle d'embeddings introuvable : {}",
  "Loading model {}…": "Chargement du modèle {}…",
  "Decoding the prompt: {e}": "Décodage du prompt : {e}",
  "Generation cancelled": "Génération annulée",
  "Decoding: {e}": "Décodage : {e}",
  "{name}: timed out": "{name} : délai dépassé",
  "Text capped at 20 MB per source.": "Texte limité à 20 Mo par source.",
  "An HTTP or HTTPS video link without credentials is required.":
    "Un lien vidéo HTTP ou HTTPS sans identifiants est requis.",
  "Whisper model missing. Install the engines, then point to the ggml file under Engines.":
    "Modèle Whisper absent. Installez les moteurs puis renseignez le fichier ggml dans Moteurs.",
  "Transcription · automatic language detection":
    "Transcription · détection automatique de la langue",
  "Cannot determine the PDF page count.":
    "Impossible de déterminer le nombre de pages du PDF.",
  "PDF capped at 10,000 pages per source.":
    "PDF limité à 10 000 pages par source.",
  "Downloading media · yt-dlp": "Téléchargement du média · yt-dlp",
  "No media downloaded": "Aucun média téléchargé",
  "Document capped at 256 MB per file.":
    "Document limité à 256 Mo par fichier.",
  "No usable text after extraction. Check that the source holds readable text.":
    "Aucun texte exploitable après extraction. Vérifiez que la source contient du texte lisible.",
  "Semantic indexing · {}/{}": "Indexation sémantique · {}/{}",
  "Lexical index ready. Embeddings incomplete: {e}":
    "Index lexical prêt. Embeddings incomplets : {e}",
  "EmbeddingGemma 300M (multilingual, light)":
    "EmbeddingGemma 300M (multilingue, léger)",
  "Qwen3-Embedding 0.6B (multilingual, more accurate, heavier)":
    "Qwen3-Embedding 0.6B (multilingue, plus précis, plus lourd)",
  "nomic-embed-text v1.5 (mostly English, very light)":
    "nomic-embed-text v1.5 (anglais surtout, très léger)",
  "Invalid address: http(s) scheme, no credentials, no parameters.":
    "Adresse invalide : schéma http(s), sans identifiants ni paramètres.",
  "Provider address incomplete.": "Adresse du fournisseur incomplète.",
  "GGUF model not found: {}": "Modèle GGUF introuvable : {}",
  "This provider requires an API key.": "Ce fournisseur exige une clé d'API.",
  "Invalid embedding response": "Réponse embeddings invalide",
  "The model returned an empty answer.":
    "Le modèle a renvoyé une réponse vide.",
  "No user message to send.": "Aucun message utilisateur à envoyer.",
  "The provider refused this request (safety filter).":
    "Le fournisseur a refusé cette requête (filtre de sécurité).",
  "Engine stream cut short; answer not saved.":
    "Flux du moteur interrompu avant la fin ; réponse non enregistrée.",
  "The verifier returned no verdict: {}":
    "Le vérificateur n'a pas renvoyé de verdict : {}",
  "Truncated archive: the file is incomplete or corrupt.":
    "Archive tronquée : le fichier est incomplet ou corrompu.",
  "No text found: empty document, or content only as images.":
    "Aucun texte trouvé : document vide, ou contenu uniquement sous forme d'images.",
  "LANGOLIER_OFFICE_FIXTURES not set": "LANGOLIER_OFFICE_FIXTURES non défini",
  "{} -> {} section(s), {total} characters":
    "{} → {} section(s), {total} caractères",
  "The question must hold between 1 and 12,000 characters.":
    "La question doit contenir entre 1 et 12 000 caractères.",
  "Searching your memory": "Recherche dans votre mémoire",
  "{abstain}\\n\\nAdd a source or narrow your question. You can also switch to Free conversation mode.":
    "{abstain}\\n\\nAjoutez une source ou précisez votre question. Vous pouvez aussi choisir le mode Conversation libre.",
  "Writing the answer": "Rédaction de la réponse",
  "Verifying the answer": "Vérification de la réponse",
  "{abstain}\\n\\n_Answer withdrawn by verification: {}_":
    "{abstain}\\n\\n_Réponse retirée par la vérification : {}_",
  "Verification failed: {e}": "Vérification impossible : {e}",
  "This assistant's daily quota is spent. Try again tomorrow.":
    "Le quota quotidien de cet assistant est atteint. Réessayez demain.",
  "Error: the local server did not start.":
    "Erreur : le serveur local n'a pas démarré.",
  "Window: {e}": "Fenêtre : {e}",
  "Bundle installed: {name}": "Bundle installé : {name}",
  "Bundle already installed.": "Bundle déjà installé.",
  "No .langolier file found in {} (nor embedded in the executable).":
    "Aucun fichier .langolier trouvé dans {} (ni embarqué dans l'exécutable).",
  "Cannot listen on {addr}: {e}": "Impossible d'écouter sur {addr} : {e}",
  "Update installed: {name}": "Mise à jour installée : {name}",
  "Update refused: {e}": "Mise à jour refusée : {e}",
  "Request: {e}": "Requête : {e}",
  "Chat model missing: {} (expected in the models/ folder).":
    "Modèle de conversation absent : {} (attendu dans le dossier models/).",
  "Embedding model missing: {} - word search only.":
    "Modèle d'embeddings absent : {} — recherche par mots seulement.",
  'Chat model "{}" missing from Ollama: `ollama pull {}`.':
    "Modèle de conversation « {} » absent d'Ollama : `ollama pull {}`.",
  'Embedding model "{}" missing from Ollama: `ollama pull {}` - word search only until then.':
    "Modèle d'embeddings « {} » absent d'Ollama : `ollama pull {}` — recherche par mots seulement en attendant.",
  "Ollama is not answering on {base}: answers will fail until it starts.":
    "Ollama ne répond pas sur {base} : les réponses échoueront jusqu'à son démarrage.",
  "Ollama is not answering on {base}: word search only.":
    "Ollama ne répond pas sur {base} : recherche par mots seulement.",
  "Ollama unreachable on {}: search will be lexical and answers will fail until it starts.":
    "Ollama injoignable sur {} : la recherche sera lexicale et les réponses échoueront jusqu'à son démarrage.",
  "Downloading model {model}…": "Téléchargement du modèle {model}…",
  "Model {model} ready.": "Modèle {model} prêt.",
  "Download of {model} refused: {}": "Téléchargement de {model} refusé : {}",
  "Cannot download {model}: {e}": "Téléchargement de {model} impossible : {e}",
  "Invalid bot token: it looks like 123456789:AAH…":
    "Jeton de bot invalide : il ressemble à 123456789:AAH…",
  "invalid response": "réponse invalide",
  "This assistant is private.": "Cet assistant est privé.",
  "Sorry, something went wrong: {e}": "Désolé, une erreur est survenue : {e}",
  "Langolier's own data folder cannot be watched.":
    "Le dossier de données de Langolier ne peut pas être surveillé.",
  "This folder is already watched.": "Ce dossier est déjà surveillé.",
  "Folder not found (volume unmounted?)":
    "Dossier introuvable (volume démonté ?)",
  "Changed in the watch": "Modifié dans la vigie",
  "Resumed after interruption": "Reprise après interruption",
  "Reindex requested": "Réindexation demandée",
  GB: "Go",
  MB: "Mo",
  KB: "Ko",
  "No compatible vector. Reindex the sources with the configured embedding model.":
    "Aucun vecteur compatible. Réindexez les sources avec le modèle d’embeddings configuré.",
  "No passage clears the relevance threshold":
    "Aucun passage ne dépasse le seuil de pertinence",
  "+ TEXT": "+ TEXTE",
  OFFICE: "BUREAUTIQUE",
  YOU: "VOUS",
  You: "Vous",
  sources: "sources",
  passages: "passages",
  vectors: "vecteurs",
  All: "Tout",
  Save: "Enregistrer",
  Delete: "Supprimer",
  "Add the watch": "Ajouter la vigie",
  "30 seconds": "30 secondes",
  "1 minute": "1 minute",
  "5 minutes": "5 minutes",
  "15 minutes": "15 minutes",
  "1 hour": "1 heure",
  "6 hours": "6 heures",
  "1 day": "1 jour",
  "7 days": "7 jours",
  "{n} day(s)": "{n} jour(s)",
  "{n} hour(s)": "{n} heure(s)",
  "{n} minute(s)": "{n} minute(s)",
  never: "jamais",
  "{n} s ago": "il y a {n} s",
  "{n} min ago": "il y a {n} min",
  "{n} h ago": "il y a {n} h",
  "{n} d ago": "il y a {n} j",
  "{n} running": "{n} en cours",
  "{n} failed": "{n} en erreur",
  "{w} watch(es), {d} source(s)": "{w} vigie(s), {d} source(s)",
  "{n} source(s) chosen out of {total}": "{n} source(s) choisie(s) sur {total}",
  Night: "Nuit",
  Light: "Clair",
  Sage: "Sauge",
  Sand: "Sable",
  Ink: "Encre",
  Coral: "Corail",
  "Remote administration": "Administration à distance",
  "Update or roll back the exported chatbot from its own page":
    "Mettre à jour ou revenir en arrière depuis la page du chatbot exporté",
  "Enable administration from the chat page":
    "Activer l’administration depuis la page de conversation",
  "A small gear appears in the page footer. Every action needs the secret below; with a wrong secret five times, the page pauses for fifteen minutes.":
    "Un petit engrenage apparaît en pied de page. Chaque action exige le secret ci-dessous ; après cinq mauvais secrets, la page se met en pause quinze minutes.",
  "Admin secret (set; type a new one to replace it)":
    "Secret d’administration (défini ; saisissez-en un nouveau pour le remplacer)",
  "Admin secret (at least 12 characters)":
    "Secret d’administration (12 caractères minimum)",
  "Secret generated: {secret}. Copy it now, it will not be shown again.":
    "Secret généré : {secret}. Copiez-le maintenant, il ne sera plus affiché.",
  Generate: "Générer",
  "Allow importing a .langolier from the page":
    "Autoriser l’import d’un .langolier depuis la page",
  "Allow restoring or deleting earlier versions from the page":
    "Autoriser la restauration ou la suppression de versions précédentes depuis la page",
  "Every bundle the chatbot ever ran is kept on the server, twenty at most, named and timestamped. Nothing is ever downloaded from the page: no export, so the corpus and the API key stay on the server. The bundle the kit shipped with cannot be deleted.":
    "Chaque bundle que le chatbot a fait tourner est conservé sur le serveur, vingt au plus, nommé et horodaté. Rien n’est jamais téléchargé depuis la page : pas d’export, le corpus et la clé d’API restent sur le serveur. Le bundle livré avec le kit ne peut pas être supprimé.",
  "The admin secret needs at least 12 characters.":
    "Le secret d’administration doit faire au moins 12 caractères.",
  "Remote administration needs a secret.":
    "L’administration à distance exige un secret.",
  "Chat model (curated tag, or path to a GGUF file)":
    "Modèle de conversation (tag de la liste, ou chemin d’un fichier GGUF)",
  "A curated tag is downloaded once into the app cache with the button below, then loaded on demand. llama.cpp runs inside Langolier: nothing to install, nothing listening on a port.":
    "Un tag de la liste se télécharge une fois dans le cache de l’application avec le bouton ci-dessous, puis se charge à la demande. llama.cpp tourne dans Langolier : rien à installer, rien qui écoute sur un port.",
  "Embedding model (curated tag, or path to a GGUF file)":
    "Modèle d’embeddings (tag de la liste, ou chemin d’un fichier GGUF)",
  Other: "Autre",
  "Curated tags land in the app cache; other names go through Ollama":
    "Les tags de la liste vont dans le cache de l’application ; les autres noms passent par Ollama",
  "Unload idle models after (minutes, 0 = never)":
    "Décharger les modèles inactifs après (minutes, 0 = jamais)",
  "Memory is released when the engine has not been used for this long, and the next question reloads the model in a few seconds.":
    "La mémoire est rendue quand le moteur n’a pas servi pendant cette durée ; la question suivante recharge le modèle en quelques secondes.",
  "Chat model not downloaded: {}. Install it from the Engines page.":
    "Modèle de conversation non téléchargé : {}. Installez-le depuis la page Moteurs.",
  "Unsupported provider": "Fournisseur non pris en charge",
  queued: "en attente",
  None: "Aucun",
  "Assistant not found": "Assistant introuvable",
  "Conversation not found": "Conversation introuvable",
  "Passage not found": "Passage introuvable",
  "Pick a destination folder.": "Choisissez un dossier de destination.",
  "Invalid rating": "Note invalide",
  "Empty question": "Question vide",
  "This file is not a Langolier bundle.":
    "Ce fichier n’est pas un bundle Langolier.",
  "The text is empty.": "Le texte est vide.",
  "Invalid search mode": "Mode de recherche invalide",
  "This conversation has reached its limit. Open a new one to continue.":
    "Cette conversation a atteint sa limite. Ouvrez une nouvelle conversation pour continuer.",
  "This path is not a folder.": "Ce chemin n’est pas un dossier.",
  "Watch not found": "Vigie introuvable",
  "This file is not a valid zip archive.":
    "Ce fichier n’est pas une archive zip valide.",
  "Empty Whisper transcription": "Transcription Whisper vide",
  "Chat page language": "Langue de la page de conversation",
  "Follow the visitor's browser": "Suivre le navigateur du visiteur",
  "Buttons, placeholders and notices of the exported chatbot page. The assistant's own answers follow its mission and the question.":
    "Boutons, textes d’aide et messages de la page du chatbot exporté. Les réponses de l’assistant suivent sa mission et la question posée.",
  "Window launcher": "Lanceur fenêtre",
  "How the exported window app sits on the desktop":
    "Comment l’app fenêtre exportée se place sur le bureau",
  "Icon in the menu bar or system tray":
    "Icône dans la barre des menus ou la zone de notification",
  "Open, Ask and Quit from the icon. Closing the window then hides it instead of quitting.":
    "Ouvrir, Demander et Quitter depuis l’icône. Fermer la fenêtre la masque alors au lieu de quitter.",
  "Start hidden, in the menu bar": "Démarrer réduit, dans la barre des menus",
  "Global shortcut for the floating question bar (empty = none)":
    "Raccourci global de la barre de question flottante (vide = aucun)",
  Clear: "Effacer",
  "Like Langolier's own palette: a floating bar over any application, Esc or a click outside closes it. Works on macOS, Windows and Linux.":
    "Comme la palette de Langolier : une barre flottante par-dessus n’importe quelle application, Échap ou un clic à l’extérieur la ferme. Fonctionne sur macOS, Windows et Linux.",
  "Langolier opens without a window; the icon and the shortcut are there.":
    "Langolier s’ouvre sans fenêtre ; l’icône et le raccourci restent disponibles.",
  "Launch at login": "Lancer à l’ouverture de session",
  "Starts hidden at login, so the palette is always one shortcut away.":
    "Démarre réduit à l’ouverture de session : la palette est toujours à un raccourci.",
  "Go on…": "Poursuivez…",
  "Copy the answer": "Copier la réponse",
  Copy: "Copier",
  Copied: "Copié",
  "Copy failed": "Copie impossible",
  "Add a source, or narrow the question.":
    "Ajoutez une source, ou précisez la question.",
  "Nothing in your sources supports an answer. Add a source, narrow the question, or switch to Free conversation.":
    "Rien dans vos sources ne permet de répondre. Ajoutez une source, précisez la question, ou passez en Conversation libre.",
};
