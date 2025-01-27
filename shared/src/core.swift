import Foundation
import SharedTypes

@MainActor
class Core: ObservableObject {
    @Published var view: CounterViewModel
    
    init() {
        let app: AppViewModel = try! .bincodeDeserialize(input: [UInt8](BitBridge.view(AppModule.counter)))
        self.view = app.counter!;
    }
    
    func update(_ event: CounterEvent) {
        let event: AppEvent = AppEvent.counter(event);
        let effects = [UInt8](processEvent(Data(try! event.bincodeSerialize())))
        
        let requests: [Request] = try! .bincodeDeserialize(input: effects)
        for request in requests {
            processEffect(request)
        }
    }
    
    func processEffect(_ request: Request) {
        switch request.effect {
        case .render:
            view = try! .bincodeDeserialize(input: [UInt8](BitBridge.view(AppModule.counter)))
        }
    }
}
