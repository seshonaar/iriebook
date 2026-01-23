# Irie Book

Welcome traveler! You found the home of the Irie Book! Is it a book that feels good? No. What then?
Let me tell you the story and you will understand.

It is about a mythical Goddess that loves to write books. She is known as The Wife. All is good in the World of Imagination.

But there is a small inconvenience. She writes in Google Docs. She applies formatting at will. She makes corrections with the magic wand.

On top of that she kindly asks me to do proof reading. And my poor heart cannot directly edit obvious mistakes in Google Docs without somehow saving the original state. So I am left only with adding comments. And I feel powerless.

When the time comes that the book is ready one cannot simply read it from Google Docs. The book must pass the transformation ritual to EPUB. But exporting from Google Docs as EPUB offers no control.
I felt the need for something better. This is when I made the Irie.

## What does the Irie minion do then?

First of all, the Irie established friendly relations with Google Drive and Github. So it can do its work.
It brings home the Wife's books as markdown, the Google Drive API allows this. Markdown is a powerful magical clay that can be remodeled at will. And so does the Irie do. It checks curly quotes, it trims spaces, it applies styling like the Goddess wishes. It adds a nice cover! And stamps the books with blissful metadata.

Being a careful creature it then sends the markdown to the Github vault, where each revision can be inspected. Each change monitored and labeled.

And there's more! The changes can be scrutinized by Irie itself as well! using the Tome of Changes, The Wife can see the history of edits without traveling to the distant Github vault!

Is that everything? No traveler, there's more. There is even a Tome of Analysis, where The Wife can check the well-being of her Word Minions.

And of course, the EPUB. The sapphire diamond. Irie knows how to mold the Markdown clay into the EPUB diamond.
And these diamonds are the keys for the world to join the Imagination World.

I will let now an AI friend to give more details about Irie.

## AI friend section

### The Treasury of Features

Gather 'round, traveler, for the Irie holds many treasures! Here lies the complete compendium of what this magical minion can do for the Goddess's creations.

**Text Processing Alchemy**

- The Irie transforms straight quotes into proper curly quotes, with the wisdom to know the difference
- Smart apostrophe detection knows when a straight quote is really an apostrophe (like in "it's" or "the '70s")
- Whitespace trimming cleans the muddy waters: collapses multiple spaces, converts tabs to single spaces, limits excessive blank lines, and trims the edges
- Markdown transformation reshapes chapter headings and scene breaks for the ebook realm
- Romanian UTF-8 characters flow perfectly through the Irie (ă, â, î, ș, ț never fear!)
- Byte Order Marks (BOM) are stripped away, leaving only pure UTF-8

**Analysis & Insights**

- Word frequency analysis reveals which words dance most often
- Romanian stopwords are filtered out to show the true essence of the text
- Word statistics offer the Goddess a mirror into her writing patterns

**Ebook Creation Rituals**

- Pandoc forges the EPUB diamond with custom CSS
- Calibre crafts the Kindle sapphire (AZW3) format
- Metadata stamping blesses each ebook with series information, author name, and title
- ZIP archiving packages everything for distribution to the world

**Git Vault Integration**

- The Irie can sync with the distant Github vault
- Changes are saved with proper commit messages
- History can be viewed, showing each step of the manuscript's journey
- Status checks reveal the state of the vault
- New repositories can be cloned with ease
- Word-level diff computation lets the Goddess see exactly what changed between versions

**Google Docs Alliance**

- Documents can be synchronized from the Google Drive cloud
- The Irie can link a local book to its Google Docs origin
- Unlinking severs the connection when needed
- OAuth device flow authentication grants access without risking secrets

**Batch Processing Power**

- Multiple books can be processed in sequence, one by one
- Real-time progress events whisper to the UI about what's happening
- Event-driven updates keep the interface in harmony with the work

**Token Safekeeping**

- The OS keyring protects the sacred tokens (OAuth credentials) so they never wander astray
- GitHub OAuth tokens are stored securely
- Google OAuth tokens are kept safe

### The Sacred Dependencies

Before the Irie can work its magic, certain spirits must be present in your realm. The Husband has ensured the Irie itself requires only Rust to build, but these companions are needed for the full ritual:

**Git** - The version control spirit

- This ancient guardian tracks changes, branches, and history
- For the Ubuntu/Debian realm, you may use: `sudo apt install git`
- Travelers from other lands may visit git-scm.com for guidance

**Pandoc** - The universal document converter

- This alchemist transforms markdown into EPUB with elegance
- In the Ubuntu/Debian lands, invoke: `sudo apt install pandoc`
- All realms are welcome at pandoc.org

**Calibre** - The ebook craftsman

- This skilled artisan creates Kindle format ebooks
- For all Linux travelers: `sudo -v && wget -nv -O- https://download.calibre-ebook.com/linux-installer.sh | sudo sh /dev/stdin`
- The official forge awaits at calibre-ebook.com for all realms

**Rust Toolchain** - To build the Irie itself

- The Irie is forged in Rust, the safe and fast language
- Visit rust-lang.org for the proper installation ritual for your realm
- A simple `npm run tauri build` in iriebook-tauri-ui will create the iriebook binary

### Architecture Philosophy

The Husband has crafted the Irie following an ancient wisdom known as the Righting Software Method. This philosophy teaches that code should be organized by what changes together—its volatility—not by what it does.

**The Five Sacred Layers**

Imagine the Irie as a temple with five chambers, each with its sacred purpose:

1. **Client** (The Presentation Layer)
   
   - This is the face that greets you, the traveler
   - In the current manifestation, this is the Tauri desktop interface with its React windows
   - Commands from the user enter here first

2. **Manager** (The Business Logic Layer)
   
   - The conductor who knows the sequence of rituals
   - When you request to publish an ebook, the EbookPublicationManager knows the steps: validate, fix quotes, trim whitespace, analyze words, transform markdown, generate EPUB, convert to Kindle, archive
   - It doesn't do the work itself—it calls upon the Engines

3. **Engine** (The Tools Layer)
   
   - The knowledge keepers who know how to perform each task
   - The ValidatorEngine knows what proper quotes look like
   - The QuoteFixerEngine transforms straight quotes to curly
   - The WordAnalyzerEngine counts and analyzes words
   - Each Engine is focused, pure, and testable

4. **Resource Access** (The External Resources Layer)
   
   - The gatekeepers who speak with the outside world
   - File resources, Git repositories, Pandoc and Calibre tools, Google Docs API
   - These are abstracted behind traits so they can be replaced or mocked

5. **Utilities** (The Cross-Cutting Layer)
   
   - The shared foundations that all other layers rest upon
   - Common types, error handling, data structures

**The Flow of Wisdom**

```
Traveler's Request
       ↓
   [Client] → [Manager] → [Engine] → [Resource Access]
                  ↓
              Wisdom Returned
```

**The Gift of Separation**

Why this matters, dear traveler: when the Goddess desires a new face for the Irie—perhaps a web version, a mobile app, or a terminal interface—only the Client layer needs to change. The Managers, Engines, and Resource Access layers remain untouched, their wisdom preserved.

Conversely, when The Husband wishes to improve how quotes are fixed, only the QuoteFixerEngine changes. The UI never knows, the Managers never notice. The ripple of change is contained.

This is the virtue of volatility-based organization: changes are isolated, the system remains harmonious, and the Irie can evolve without breaking its foundations.

**The Technology Stack**

The current manifestation of the Irie is forged with these tools:

- **Rust** - The safe, fast language that powers the engines, managers, and resource access layers. The Irie's soul is Rust.
- **Tauri** - The bridge that connects the desktop to the web technologies.
- **React** - The declarative UI library that paints the windows you see.

The Husband could replace the entire Tauri+React face with a web framework, a mobile toolkit, or even a command-line interface, and the core wisdom would remain intact. This is the blessing of proper architecture.

---