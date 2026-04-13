import Foundation

/// Temperature unit for weather readings.
public enum TemperatureUnit {
    case celsius
    case fahrenheit
}

/// A weather observation at a point in time.
public struct WeatherReading {
    public let temperature: Double
    public let unit: TemperatureUnit
    public let humidity: Double
    public let timestamp: Date

    /// Convert temperature to the other unit.
    public func converted(to targetUnit: TemperatureUnit) -> WeatherReading {
        if unit == targetUnit { return self }
        let newTemp = unit == .celsius
            ? temperature * 9.0 / 5.0 + 32.0
            : (temperature - 32.0) * 5.0 / 9.0
        return WeatherReading(temperature: newTemp, unit: targetUnit, humidity: humidity, timestamp: timestamp)
    }
}

/// Protocol for weather data providers.
public protocol WeatherProvider {
    func currentWeather(for city: String) async throws -> WeatherReading
    func forecast(for city: String, days: Int) async throws -> [WeatherReading]
}

/// Service that aggregates weather data from multiple providers.
public class WeatherService {
    private let providers: [WeatherProvider]

    public init(providers: [WeatherProvider]) {
        self.providers = providers
    }

    /// Get the average temperature from all providers.
    public func averageTemperature(for city: String) async throws -> Double {
        var total = 0.0
        var count = 0
        for provider in providers {
            let reading = try await provider.currentWeather(for: city)
            total += reading.converted(to: .celsius).temperature
            count += 1
        }
        return count > 0 ? total / Double(count) : 0.0
    }
}
