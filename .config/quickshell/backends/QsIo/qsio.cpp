#include "qsio.hpp"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJSEngine>
#include <QQmlEngine>
#include <QQmlExtensionPlugin>
#include <QSaveFile>
#include <QDebug>
#include <qqml.h>

QsIo::QsIo(QObject* parent): QObject(parent) {}

static void ensureParentDir(const QString& path) {
	const auto parent = QFileInfo(path).absolutePath();
	if (!parent.isEmpty()) QDir().mkpath(parent);
}

void QsIo::writeFile(const QString& path, const QString& content) {
	ensureParentDir(path);

	QSaveFile file(path);
	if (!file.open(QIODevice::WriteOnly)) {
		qWarning() << "QsIo: failed to open" << path << file.errorString();
		return;
	}

	file.write(content.toUtf8());
	if (!file.commit()) {
		qWarning() << "QsIo: failed to commit" << path << file.errorString();
	}
}

void QsIo::appendFile(const QString& path, const QString& content) {
	ensureParentDir(path);

	QFile file(path);
	if (!file.open(QIODevice::Append)) {
		qWarning() << "QsIo: failed to open" << path << file.errorString();
		return;
	}

	file.write(content.toUtf8());
}

QString QsIo::readFile(const QString& path) {
	QFile file(path);
	if (!file.open(QIODevice::ReadOnly)) return QString();
	return QString::fromUtf8(file.readAll());
}

// Classic loadable QML plugin: registers QsIo as the "QsIo" singleton.
class QsIoPlugin : public QQmlExtensionPlugin {
	Q_OBJECT;
	Q_PLUGIN_METADATA(IID QQmlExtensionInterface_iid);

public:
	void registerTypes(const char* uri) override {
		qmlRegisterSingletonType<QsIo>(uri, 1, 0, "QsIo", [](QQmlEngine*, QJSEngine*) {
			return new QsIo();
		});
	}
};

#include "qsio.moc"
