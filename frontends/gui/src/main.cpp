#include <QApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QIcon>
#include "app.h"

int main(int argc, char *argv[]) {
    QApplication app(argc, argv);
    app.setApplicationName("Synthread");
    app.setApplicationVersion("0.1.0");
    app.setOrganizationName("Synthread");

    // Register C++ types with QML
    qmlRegisterType<SynthreadApp>("Synthread", 1, 0, "SynthreadApp");

    SynthreadApp synthreadApp;

    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty("synthread", &synthreadApp);

    // Add Qt6 QML import path (required on some systems)
    engine.addImportPath("/usr/lib/x86_64-linux-gnu/qt6/qml");

    engine.load(QUrl(QStringLiteral("qrc:/main.qml")));

    if (engine.rootObjects().isEmpty()) {
        return -1;
    }

    return app.exec();
}
