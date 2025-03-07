//
//  Modal.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 6/3/25.
//

import SwiftUI
import Foundation
import SharedTypes

enum ShareSelection: String {
    case People
    case PublicUrl
    case Devices
}

extension TransferMethodSelection {
    static var allCases: [TransferMethodSelection] {
        [.device, .user, .internet]
    }
    
    var name: String {
        get {
            switch self {
            case .device: return "Your devices"
            case .user: return "People"
            case .internet: return "Public URL"
            }
        }
    }
    
    var icon: Image {
        get {
            switch self {
            case .device: return ImageAsset.DeviceEmpty.image
            case .user: return ImageAsset.UserEmpty.image
            case .internet: return ImageAsset.GlobeEmpty.image
            }
        }
    }
    
    var body: AnyView {
        get {
            switch self {
            case .device: return AnyView(DeviceShareView())
            case .user: return AnyView(PeopleShareView())
            case .internet: return AnyView(PublicUrlShareView())
            }
        }
    }
}

struct ShareModal: View {
    @EnvironmentObject var core: Core
    @State private var selection = TransferMethodSelection.user
    let selections = TransferMethodSelection.allCases
    
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack(spacing: 15) {
                selection.icon
                    .foregroundStyle(Theme.GreenSecondary.color)
                    .padding(.all, 8)
                    .clipShape(Circle())
                    .background(Circle().foregroundStyle(Theme.BluePrimary.color.opacity(0.1)))
                
                HStack {
                    Menu(selection.name) {
                        Button(TransferMethodSelection.device.name, action: {
                            selection = .device
                        })
                        Button(TransferMethodSelection.user.name, action: {
                            selection = .user
                        })
                        Button(TransferMethodSelection.internet.name, action: {
                            selection = .internet
                        })
                    }
                    .modifier(Body1())
                    .foregroundStyle(Theme.PrimaryText.color)
                    .background(.clear)
                    
                    ImageAsset.ArrowDown.image
                        .resizable()
                        .frame(width: 25, height: 25)
                }
                
                Spacer()
            }
            
            selection.body
                .frame(width: .infinity)
        }
        .frame(minWidth: 300, maxWidth: 800)
        .padding()
        .background(Theme.BlackBase.color.opacity(0.2))
        .clipShape(RoundedRectangle(cornerRadius: 30))
        .overlay(RoundedRectangle(cornerRadius: 30).strokeBorder(Theme.PrimaryText.color.opacity(0.1), lineWidth: 1))
        .shadow(radius: 2)
    }
}

#Preview {
    ZStack {
        ShareModal()
            .previewLayout(.sizeThatFits)
            .frame(width: .infinity, height: 350)
            .environmentObject(CoreMock() as Core)
    }
    .background(Theme.BlackBase.color)
}
